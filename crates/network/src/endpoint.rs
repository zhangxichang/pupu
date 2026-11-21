use std::{collections::HashMap, fmt, sync::Mutex};

use eyre::{Result, eyre};
use iroh::{
    Endpoint, EndpointAddr, EndpointId, RelayMode, RelayUrl, SecretKey,
    address_lookup::memory::MemoryLookup,
    endpoint::{Connection, presets::Preset},
    protocol::{AcceptError, ProtocolHandler, Router},
};
use iroh_blobs::{BlobsProtocol, Hash, store::mem::MemStore};
use tokio::sync::mpsc;

use crate::{
    ALPN_PUPU,
    events::NetworkEvent,
    identity::verify_envelope,
    protocol::{
        ERROR_INVALID_REQUEST, ERROR_UNSUPPORTED, ERROR_UNSUPPORTED_VERSION, Hello,
        PROTOCOL_VERSION, Profile, ProtocolError, Request, read_frame, write_frame,
    },
};

/// Configuration for a network node.
pub struct NetworkConfig {
    pub secret_key: SecretKey,
    pub profile: Profile,
    pub use_relay: bool,
    pub relay_url: Option<RelayUrl>,
}

impl fmt::Debug for NetworkConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NetworkConfig")
            .field("use_relay", &self.use_relay)
            .field("relay_url", &self.relay_url)
            .field("profile", &self.profile)
            .finish()
    }
}

/// A running iroh node serving the Pupu control protocol.
pub struct Network {
    endpoint: Endpoint,
    router: Router,
    pool: ConnectionPool,
    profile: Profile,
    store: MemStore,
}

impl Network {
    /// Start an endpoint and the `pupu/chat/1` protocol handler.
    pub async fn start(config: NetworkConfig) -> Result<(Self, mpsc::Receiver<NetworkEvent>)> {
        let lookup = MemoryLookup::new();
        let mut builder =
            Endpoint::builder(iroh::endpoint::presets::Minimal).address_lookup(lookup.clone());
        let has_relay = config.relay_url.is_some() || config.use_relay;
        if let Some(relay_url) = config.relay_url.clone() {
            builder = builder.relay_mode(RelayMode::custom([relay_url]));
        } else if config.use_relay {
            builder = iroh::endpoint::presets::N0.apply(builder);
        } else {
            builder = builder.relay_mode(RelayMode::Disabled);
        }
        builder = builder.secret_key(config.secret_key.clone());
        let endpoint = builder.bind().await?;
        if has_relay {
            endpoint.online().await;
        }

        let mut profile = config.profile;
        profile.relay_url = endpoint.addr().relay_urls().next().map(ToString::to_string);
        let store = MemStore::new();
        let blobs = BlobsProtocol::new(&store, None);
        let (events, receiver) = mpsc::channel(256);
        let handler = NetworkHandler {
            profile: profile.clone(),
            events,
        };
        let router = Router::builder(endpoint.clone())
            .accept(ALPN_PUPU, handler)
            .accept(iroh_blobs::ALPN, blobs)
            .spawn();
        let pool = ConnectionPool::new(endpoint.clone(), lookup);

        Ok((
            Self {
                endpoint,
                router,
                pool,
                profile,
                store,
            },
            receiver,
        ))
    }

    pub fn id(&self) -> EndpointId {
        self.endpoint.id()
    }

    pub fn addr(&self) -> EndpointAddr {
        self.endpoint.addr()
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Add bytes to the in-memory blob store and return their BLAKE3 address.
    pub async fn add_blob(&self, bytes: Vec<u8>) -> Result<[u8; 32]> {
        let tag = self.store.blobs().add_bytes(bytes).await?;
        Ok(*tag.hash.as_bytes())
    }

    /// Fetch a blob from `provider` into the in-memory store.
    pub async fn download_blob(&self, hash: [u8; 32], provider: EndpointId) -> Result<()> {
        let downloader = self.store.downloader(&self.endpoint);
        downloader
            .download(Hash::from_bytes(hash), [provider])
            .await?;
        Ok(())
    }

    pub async fn read_blob(&self, hash: [u8; 32]) -> Result<Vec<u8>> {
        Ok(self
            .store
            .blobs()
            .get_bytes(Hash::from_bytes(hash))
            .await?
            .to_vec())
    }

    /// Add a peer address to the in-memory address book.
    pub fn add_peer(&self, addr: EndpointAddr) {
        self.pool.lookup.add_endpoint_info(addr);
    }

    /// Exchange Hello with a peer and return its profile.
    pub async fn request_to(&self, id: EndpointId, request: Request) -> Result<Request> {
        self.request(EndpointAddr::new(id), request).await
    }

    pub async fn hello_to(&self, id: EndpointId) -> Result<Profile> {
        match self
            .request_to(
                id,
                Request::Hello(Hello {
                    version: PROTOCOL_VERSION,
                    profile: self.profile.clone(),
                }),
            )
            .await?
        {
            Request::HelloAck(hello) => Ok(hello.profile),
            Request::Error(error) => Err(eyre!("对方拒绝握手：{}", error.message)),
            other => Err(eyre!("握手响应不正确：{other:?}")),
        }
    }

    pub async fn hello(&self, addr: EndpointAddr) -> Result<Profile> {
        match self
            .request(
                addr,
                Request::Hello(Hello {
                    version: PROTOCOL_VERSION,
                    profile: self.profile.clone(),
                }),
            )
            .await?
        {
            Request::HelloAck(hello) => Ok(hello.profile),
            Request::Error(error) => Err(eyre!("对方拒绝握手：{}", error.message)),
            other => Err(eyre!("握手响应不正确：{other:?}")),
        }
    }

    pub async fn ping(&self, addr: EndpointAddr) -> Result<()> {
        match self.request(addr, Request::Ping).await? {
            Request::Pong => Ok(()),
            other => Err(eyre!("Ping 响应不正确：{other:?}")),
        }
    }

    /// Send one request on a pooled connection, reconnecting once on failure.
    pub async fn request(&self, addr: EndpointAddr, request: Request) -> Result<Request> {
        n0_future::time::timeout(
            std::time::Duration::from_secs(15),
            self.request_inner(addr, request),
        )
        .await
        .map_err(|_| eyre!("网络请求超时"))?
    }

    async fn request_inner(&self, addr: EndpointAddr, request: Request) -> Result<Request> {
        let id = addr.id;
        for attempt in 0..2 {
            let connection = self.pool.connection(addr.clone()).await?;
            match exchange(&connection, &request).await {
                Ok(response) => return Ok(response),
                Err(_) if attempt == 0 => {
                    self.pool.remove(id);
                }
                Err(error) => return Err(error),
            }
        }
        Err(eyre!("连接失败"))
    }

    /// Stable id of the pooled connection to `addr` (for diagnostics and tests).
    pub async fn connection_id(&self, addr: EndpointAddr) -> Result<usize> {
        Ok(self.pool.connection(addr).await?.stable_id())
    }

    pub async fn shutdown(self) -> Result<()> {
        self.router.shutdown().await?;
        self.endpoint.close().await;
        Ok(())
    }
}

struct ConnectionPool {
    endpoint: Endpoint,
    lookup: MemoryLookup,
    connections: Mutex<HashMap<EndpointId, Connection>>,
}

impl ConnectionPool {
    fn new(endpoint: Endpoint, lookup: MemoryLookup) -> Self {
        Self {
            endpoint,
            lookup,
            connections: Mutex::new(HashMap::new()),
        }
    }

    async fn connection(&self, addr: EndpointAddr) -> Result<Connection> {
        if let Some(connection) = self
            .connections
            .lock()
            .ok()
            .and_then(|connections| connections.get(&addr.id).cloned())
            && connection.close_reason().is_none()
        {
            return Ok(connection);
        }
        let connection = self.endpoint.connect(addr, ALPN_PUPU).await?;
        if let Ok(mut connections) = self.connections.lock() {
            connections.insert(connection.remote_id(), connection.clone());
        }
        Ok(connection)
    }

    fn remove(&self, id: EndpointId) {
        if let Ok(mut connections) = self.connections.lock() {
            connections.remove(&id);
        }
    }
}

#[derive(Debug, Clone)]
struct NetworkHandler {
    profile: Profile,
    events: mpsc::Sender<NetworkEvent>,
}

impl NetworkHandler {
    async fn emit(&self, event: NetworkEvent) {
        let _ = self.events.send(event).await;
    }

    async fn handle(&self, remote: EndpointId, request: Request) -> Request {
        match request {
            Request::Hello(hello) if hello.version != PROTOCOL_VERSION => {
                Request::Error(ProtocolError {
                    code: ERROR_UNSUPPORTED_VERSION,
                    message: format!("不支持的协议版本：{}", hello.version),
                })
            }
            Request::Hello(hello) => {
                self.emit(NetworkEvent::Hello {
                    endpoint_id: *remote.as_bytes(),
                    profile: hello.profile,
                })
                .await;
                Request::HelloAck(Hello {
                    version: PROTOCOL_VERSION,
                    profile: self.profile.clone(),
                })
            }
            Request::Ping => Request::Pong,
            Request::ContactRequest { profile, message } => {
                self.emit(NetworkEvent::ContactRequest {
                    endpoint_id: *remote.as_bytes(),
                    profile,
                    message,
                })
                .await;
                Request::Ok
            }
            Request::ContactAccept { profile } => {
                self.emit(NetworkEvent::ContactAccept {
                    endpoint_id: *remote.as_bytes(),
                    profile,
                })
                .await;
                Request::Ok
            }
            Request::ContactReject { reason } => {
                self.emit(NetworkEvent::ContactReject {
                    endpoint_id: *remote.as_bytes(),
                    reason,
                })
                .await;
                Request::Ok
            }
            Request::Message(envelope) => {
                if envelope.sender != *remote.as_bytes() {
                    return Request::Error(ProtocolError {
                        code: ERROR_INVALID_REQUEST,
                        message: "消息发送者与连接节点不一致".to_string(),
                    });
                }
                if verify_envelope(&envelope).is_err() {
                    return Request::Error(ProtocolError {
                        code: ERROR_INVALID_REQUEST,
                        message: "消息签名验证失败".to_string(),
                    });
                }
                let uid = envelope.uid;
                self.emit(NetworkEvent::Message {
                    endpoint_id: *remote.as_bytes(),
                    envelope,
                })
                .await;
                Request::Ack { uid }
            }
            Request::SyncRequest {
                conversation_id,
                known,
            } => {
                self.emit(NetworkEvent::SyncRequest {
                    endpoint_id: *remote.as_bytes(),
                    conversation_id,
                    known,
                })
                .await;
                Request::Ok
            }
            Request::SyncResponse { messages, has_more } => {
                self.emit(NetworkEvent::SyncResponse {
                    endpoint_id: *remote.as_bytes(),
                    messages,
                    has_more,
                })
                .await;
                Request::Ok
            }
            Request::Receipt {
                conversation_id,
                last_read_uid,
            } => {
                self.emit(NetworkEvent::Receipt {
                    endpoint_id: *remote.as_bytes(),
                    conversation_id,
                    last_read_uid,
                })
                .await;
                Request::Ok
            }
            Request::GroupInvite(invite) => {
                self.emit(NetworkEvent::GroupInvite {
                    endpoint_id: *remote.as_bytes(),
                    invite,
                })
                .await;
                Request::Ok
            }
            Request::Delivered { uid } => {
                self.emit(NetworkEvent::Delivered {
                    endpoint_id: *remote.as_bytes(),
                    uid,
                })
                .await;
                Request::Ok
            }
            other => Request::Error(ProtocolError {
                code: ERROR_UNSUPPORTED,
                message: format!("暂不支持该请求：{other:?}"),
            }),
        }
    }
}

impl ProtocolHandler for NetworkHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = connection.remote_id();
        self.emit(NetworkEvent::Connected {
            endpoint_id: *remote.as_bytes(),
        })
        .await;
        loop {
            let Ok((mut send, mut recv)) = connection.accept_bi().await else {
                break;
            };
            let Ok(request) = read_frame(&mut recv).await else {
                break;
            };
            let response = self.handle(remote, request).await;
            if write_frame(&mut send, &response).await.is_err() {
                break;
            }
            let _ = send.finish();
        }
        self.emit(NetworkEvent::Disconnected {
            endpoint_id: *remote.as_bytes(),
        })
        .await;
        Ok(())
    }
}

async fn exchange(connection: &Connection, request: &Request) -> Result<Request> {
    let (mut send, mut recv) = connection.open_bi().await?;
    write_frame(&mut send, request).await?;
    send.finish()?;
    read_frame(&mut recv).await
}
