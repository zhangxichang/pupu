use futures_lite::StreamExt;
use iroh::EndpointId;
use iroh_gossip::{
    TopicId,
    api::{GossipReceiver, GossipSender},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use wasm_bindgen::{JsError, JsValue, prelude::wasm_bindgen};

use crate::traits::JsErrorExt;

#[derive(Serialize, Deserialize)]
pub struct Ticket {
    pub id: TopicId,
    pub bootstrap: Vec<EndpointId>,
}

#[wasm_bindgen]
pub struct PersonProtocolEvent(person_protocol::Event);
impl PersonProtocolEvent {
    pub fn new(inner: person_protocol::Event) -> Self {
        Self(inner)
    }
}
#[wasm_bindgen]
impl PersonProtocolEvent {
    pub fn kind(&self) -> String {
        self.0.to_string()
    }
    pub fn as_friend_request(self) -> Result<FriendRequest, JsError> {
        if let person_protocol::Event::FriendRequest(value) = self.0 {
            Ok(FriendRequest(value))
        } else {
            Err(JsError::new("事件类型错误"))
        }
    }
    pub fn as_chat_request(self) -> Result<ChatRequest, JsError> {
        if let person_protocol::Event::ChatRequest(value) = self.0 {
            Ok(ChatRequest(value))
        } else {
            Err(JsError::new("事件类型错误"))
        }
    }
}

#[wasm_bindgen]
pub struct Person(person_protocol::Person);
impl Person {
    pub fn new(inner: person_protocol::Person) -> Self {
        Self(inner)
    }
    pub fn inner(self) -> person_protocol::Person {
        self.0
    }
}
#[wasm_bindgen]
impl Person {
    pub fn from_object(value: JsValue) -> Result<Self, JsError> {
        Ok(Self(serde_wasm_bindgen::from_value::<
            person_protocol::Person,
        >(value)?))
    }
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        Ok(serde_wasm_bindgen::to_value(&self.0)?)
    }
}

#[wasm_bindgen]
pub struct FriendRequest(person_protocol::FriendRequest);
#[wasm_bindgen]
impl FriendRequest {
    pub fn remote_id(&self) -> String {
        self.0.remote_id().to_string()
    }
    pub fn accept(self) -> Result<(), JsError> {
        self.0.accept().m()
    }
    pub fn reject(self) -> Result<(), JsError> {
        self.0.reject().m()
    }
}

#[wasm_bindgen]
pub struct ChatRequest(person_protocol::ChatRequest);
#[wasm_bindgen]
impl ChatRequest {
    pub fn remote_id(&self) -> String {
        self.0.remote_id().to_string()
    }
    pub fn accept(self) -> Result<Connection, JsError> {
        Ok(Connection(self.0.accept().m()?))
    }
    pub fn reject(self) -> Result<(), JsError> {
        self.0.reject().m()
    }
}

#[wasm_bindgen]
pub struct Connection(iroh::endpoint::Connection);
impl Connection {
    pub fn new(inner: iroh::endpoint::Connection) -> Self {
        Self(inner)
    }
}
#[wasm_bindgen]
impl Connection {
    pub async fn send(&self, message: String) -> Result<(), JsError> {
        let mut send = self.0.open_uni().await?;
        send.write_all(message.as_bytes()).await?;
        send.finish()?;
        Ok(())
    }
    pub async fn recv(&self) -> Result<Option<String>, JsError> {
        if let Ok(mut recv) = self.0.accept_uni().await {
            if let Ok(message) = recv.read_to_end(usize::MAX).await {
                return Ok(Some(String::from_utf8(message)?));
            }
        }
        Ok(None)
    }
}

#[wasm_bindgen]
pub struct Group(GossipSender, Mutex<GossipReceiver>);
impl Group {
    pub fn new(s: GossipSender, r: Mutex<GossipReceiver>) -> Self {
        Self(s, r)
    }
}
#[wasm_bindgen]
impl Group {
    pub async fn next_event(&self) -> Result<Option<JsValue>, JsError> {
        let Some(event) = self.1.lock().await.try_next().await? else {
            return Ok(None);
        };
        Ok(Some(serde_wasm_bindgen::to_value(&event)?))
    }
    pub async fn send(&self, message: String) -> Result<(), JsError> {
        self.0.broadcast(message.into()).await?;
        Ok(())
    }
}
