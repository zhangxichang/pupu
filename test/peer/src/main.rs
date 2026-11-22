use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use eyre::Result;
use runtime::{RelayUrl, Runtime};
use test_protocol::{
    ContactAccept, ContactRequest, ContactView, Identity, MessageView, Netem, OkResponse,
    PeerState, SendMessage,
};

struct Peer {
    runtime: Runtime,
    relay_url: RelayUrl,
    allow_netem: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let data_dir = std::env::var("PUPU_DATA_DIR").unwrap_or_else(|_| "/data".to_string());
    let name = std::env::var("PUPU_NAME").unwrap_or_else(|_| "peer".to_string());
    let relay_url: RelayUrl = std::env::var("PUPU_RELAY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3340".to_string())
        .parse()?;
    let control_addr: SocketAddr = std::env::var("PUPU_CONTROL_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:18080".to_string())
        .parse()?;
    let allow_netem = std::env::var("PUPU_ALLOW_NETEM").is_ok_and(|value| value == "1");

    let (runtime, mut events) =
        Runtime::open(PathBuf::from(data_dir), name, Some(relay_url.clone())).await?;
    let peer = Arc::new(Peer {
        runtime,
        relay_url,
        allow_netem,
    });

    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            log::debug!("运行时事件：{event:?}");
        }
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/identity", get(identity_handler))
        .route("/state", get(state_handler))
        .route("/contact/request", post(contact_request))
        .route("/contact/accept", post(contact_accept))
        .route("/message", post(send_message_handler))
        .route("/netem", post(netem_handler))
        .with_state(peer.clone());

    let listener = tokio::net::TcpListener::bind(control_addr).await?;
    log::info!(
        "测试节点已启动：{} ({}), control {control_addr}",
        peer.runtime.name(),
        encode_id(peer.runtime.endpoint_id())
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("无法注册 SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn internal(error: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        }
    }

    fn bad_request(error: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

impl From<eyre::Report> for ApiError {
    fn from(error: eyre::Report) -> Self {
        Self::internal(error)
    }
}

fn encode_id(bytes: impl AsRef<[u8]>) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn parse_endpoint(value: &str) -> Result<[u8; 32], ApiError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(ApiError::bad_request)?;
    bytes
        .try_into()
        .map_err(|_| ApiError::bad_request("节点 ID 长度必须为 32 字节"))
}

async fn health(State(peer): State<Arc<Peer>>) -> Result<Json<OkResponse>, ApiError> {
    if peer.runtime.ready().await {
        Ok(Json(OkResponse { ok: true }))
    } else {
        Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: "网络尚未就绪".to_string(),
        })
    }
}

async fn identity_handler(State(peer): State<Arc<Peer>>) -> Json<Identity> {
    Json(Identity {
        name: peer.runtime.name().to_string(),
        endpoint_id: encode_id(peer.runtime.endpoint_id()),
        relay_url: Some(peer.relay_url.to_string()),
    })
}

async fn state_handler(State(peer): State<Arc<Peer>>) -> Result<Json<PeerState>, ApiError> {
    let snapshot = peer.runtime.snapshot().await?;
    let contacts = snapshot
        .contacts
        .into_iter()
        .map(|contact| ContactView {
            endpoint_id: contact.endpoint_id.map(encode_id).unwrap_or_default(),
            name: contact.name,
            state: contact.state,
        })
        .collect();
    let messages = snapshot
        .messages
        .into_iter()
        .map(|message| MessageView {
            conversation_id: message.conversation_id,
            uid: encode_id(message.uid),
            sender_id: message.sender_id,
            markdown: message.markdown,
            status: message.status,
        })
        .collect();
    Ok(Json(PeerState { contacts, messages }))
}

async fn contact_request(
    State(peer): State<Arc<Peer>>,
    Json(request): Json<ContactRequest>,
) -> Result<Json<OkResponse>, ApiError> {
    let endpoint_id = parse_endpoint(&request.target)?;
    peer.runtime
        .contact_request(endpoint_id, request.message)
        .await?;
    Ok(Json(OkResponse { ok: true }))
}

async fn contact_accept(
    State(peer): State<Arc<Peer>>,
    Json(request): Json<ContactAccept>,
) -> Result<Json<OkResponse>, ApiError> {
    let endpoint_id = parse_endpoint(&request.target)?;
    peer.runtime.accept_contact(endpoint_id).await?;
    Ok(Json(OkResponse { ok: true }))
}

async fn send_message_handler(
    State(peer): State<Arc<Peer>>,
    Json(request): Json<SendMessage>,
) -> Result<Json<OkResponse>, ApiError> {
    let endpoint_id = parse_endpoint(&request.target)?;
    peer.runtime
        .send_message(endpoint_id, request.markdown)
        .await?;
    Ok(Json(OkResponse { ok: true }))
}

async fn netem_handler(
    State(peer): State<Arc<Peer>>,
    Json(request): Json<Netem>,
) -> Result<Json<OkResponse>, ApiError> {
    if !peer.allow_netem {
        return Err(ApiError::bad_request(
            "netem 未启用，请设置 PUPU_ALLOW_NETEM=1",
        ));
    }
    apply_netem(&request).await?;
    Ok(Json(OkResponse { ok: true }))
}

async fn apply_netem(netem: &Netem) -> Result<(), ApiError> {
    let interface = default_interface().await?;
    if netem.delay_ms.is_none() && netem.loss_percent.is_none() && netem.rate_kbit.is_none() {
        let output = tokio::process::Command::new("tc")
            .args(["qdisc", "del", "dev", &interface, "root"])
            .output()
            .await
            .map_err(ApiError::internal)?;
        if !output.status.success() {
            log::warn!(
                "清除 netem 失败：{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        return Ok(());
    }

    let mut command = tokio::process::Command::new("tc");
    command.args(["qdisc", "replace", "dev", &interface, "root", "netem"]);
    if let Some(delay_ms) = netem.delay_ms {
        command.arg("delay").arg(format!("{delay_ms}ms"));
        if let Some(jitter_ms) = netem.jitter_ms.filter(|jitter| *jitter > 0) {
            command.arg(format!("{jitter_ms}ms"));
        }
    }
    if let Some(loss_percent) = netem.loss_percent {
        command.arg("loss").arg(format!("{loss_percent}%"));
    }
    if let Some(rate_kbit) = netem.rate_kbit {
        command.arg("rate").arg(format!("{rate_kbit}kbit"));
    }
    let output = command.output().await.map_err(ApiError::internal)?;
    if !output.status.success() {
        return Err(ApiError::internal(format!(
            "设置 netem 失败：{}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

async fn default_interface() -> Result<String, ApiError> {
    let output = tokio::process::Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .await
        .map_err(ApiError::internal)?;
    if !output.status.success() {
        return Err(ApiError::internal(format!(
            "读取默认路由失败：{}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut parts = stdout.split_whitespace();
    while let Some(part) = parts.next() {
        if part == "dev" {
            return parts
                .next()
                .map(str::to_string)
                .ok_or_else(|| ApiError::internal("默认路由缺少网卡"));
        }
    }
    Err(ApiError::internal("找不到默认网卡"))
}
