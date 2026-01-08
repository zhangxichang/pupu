use std::sync::Arc;

use base64::{Engine, prelude::BASE64_STANDARD};
use endpoint::Ticket;
use eyre::{Result, eyre};
use iroh::{
    RelayConfig, RelayMode, SecretKey, Watcher,
    discovery::{
        dns::DnsDiscovery,
        mdns::MdnsDiscovery,
        pkarr::{PkarrPublisher, dht::DhtDiscovery},
    },
    endpoint::{Connection, ConnectionType},
    protocol::Router,
};
use iroh_blobs::{BlobsProtocol, store::fs::FsStore};
use iroh_gossip::{Gossip, api::GossipTopic};
use iroh_relay::RelayQuicConfig;
use parking_lot::Mutex;
use person_protocol::{Person, PersonProtocol};
use sharded_slab::Slab;

#[derive(Clone)]
pub struct Endpoint {
    router: Router,
    person_protocol: PersonProtocol,
    gossip_protocol: Gossip,
    _blobs_protocol: BlobsProtocol,
    connection_pool: Arc<Slab<Connection>>,
    person_protocol_event_receiver: async_channel::Receiver<person_protocol::Event>,
    person_protocol_event_next: Arc<Mutex<Option<person_protocol::Event>>>,
}
impl Endpoint {
    pub async fn new(secret_key: Vec<u8>, person: Person) -> Result<Self> {
        let (person_protocol_event_sender, person_protocol_event_receiver) =
            async_channel::unbounded();
        let relay_map = RelayMode::Default.relay_map();
        relay_map.insert(
            "https://dev.zhangxichang.com:10281".parse()?,
            RelayConfig {
                url: "https://dev.zhangxichang.com:10281".parse()?,
                quic: Some(RelayQuicConfig { port: 10282 }),
            }
            .into(),
        );
        let endpoint = iroh::Endpoint::empty_builder(RelayMode::Custom(relay_map))
            .discovery(PkarrPublisher::n0_dns())
            .discovery(DnsDiscovery::n0_dns())
            .discovery(MdnsDiscovery::builder())
            .discovery(DhtDiscovery::builder())
            .secret_key(SecretKey::from_bytes(secret_key.as_slice().try_into()?))
            .bind()
            .await?;
        let person_protocol =
            PersonProtocol::new(endpoint.clone(), person, person_protocol_event_sender);
        let gossip_protocol = Gossip::builder().spawn(endpoint.clone());
        let blobs_protocol = BlobsProtocol::new(
            &FsStore::load("store")
                .await
                .map_err(|err| eyre!(err))?
                .into(),
            None,
        );
        let router = Router::builder(endpoint)
            .accept(person_protocol::ALPN, person_protocol.clone())
            .accept(iroh_gossip::ALPN, gossip_protocol.clone())
            .accept(iroh_blobs::ALPN, blobs_protocol.clone())
            .spawn();
        Ok(Self {
            router,
            person_protocol,
            gossip_protocol,
            _blobs_protocol: blobs_protocol,
            connection_pool: Default::default(),
            person_protocol_event_receiver,
            person_protocol_event_next: Default::default(),
        })
    }
    pub async fn close(&self) -> Result<()> {
        self.router.shutdown().await?;
        Ok(())
    }
    pub fn id(&self) -> String {
        self.router.endpoint().id().to_string()
    }
    pub async fn person_protocol_event_next(&self) -> Result<person_protocol::Event> {
        Ok(self.person_protocol_event_receiver.recv().await?)
    }
    pub async fn request_person(&self, id: String) -> Result<Person> {
        Ok(self.person_protocol.request_person(id.parse()?).await?)
    }
    pub async fn request_friend(&self, id: String) -> Result<bool> {
        Ok(self.person_protocol.request_friend(id.parse()?).await?)
    }
    pub async fn request_chat(&self, id: String) -> Result<Option<Connection>> {
        Ok(self.person_protocol.request_chat(id.parse()?).await?)
    }
    pub fn conn_type(&self, id: String) -> Result<Option<String>> {
        Ok(self
            .router
            .endpoint()
            .conn_type(id.parse()?)
            .map(|mut value| {
                match value.get() {
                    ConnectionType::Direct(_) => "Direct",
                    ConnectionType::Relay(_) => "Relay",
                    ConnectionType::Mixed(_, _) => "Mixed",
                    ConnectionType::None => "None",
                }
                .to_string()
            }))
    }
    pub fn latency(&self, id: String) -> Result<Option<usize>> {
        Ok(self
            .router
            .endpoint()
            .latency(id.parse()?)
            .map(|v| v.as_millis() as _))
    }
    pub async fn subscribe_group(&self, ticket: String) -> Result<GossipTopic> {
        let ticket = serde_json::from_slice::<Ticket>(&BASE64_STANDARD.decode(ticket)?)?;
        Ok(self
            .gossip_protocol
            .subscribe(ticket.id, ticket.bootstrap)
            .await?
            .into())
    }
}
