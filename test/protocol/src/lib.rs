use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub endpoint_id: String,
    pub relay_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactView {
    pub endpoint_id: String,
    pub name: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageView {
    pub conversation_id: String,
    pub uid: String,
    pub sender_id: String,
    pub markdown: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerState {
    pub contacts: Vec<ContactView>,
    pub messages: Vec<MessageView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactRequest {
    pub target: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactAccept {
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessage {
    pub target: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Netem {
    pub delay_ms: Option<u32>,
    pub jitter_ms: Option<u32>,
    pub loss_percent: Option<f64>,
    pub rate_kbit: Option<u32>,
}
