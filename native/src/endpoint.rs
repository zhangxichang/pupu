pub mod chat_request;
pub mod connection;
pub mod friend_request;

use std::time::Duration;

use endpoint::service::{ChatRequest, FriendRequest, Person};
use iroh::{
    SecretKey,
    endpoint::{Connection, ConnectionType},
};
use slab::Slab;
use tokio::sync::{Mutex, mpsc};

use crate::{
    error::{Error, OptionGet, OptionGetClone},
    state::State,
};

#[derive(Default)]
pub struct Endpoint {
    inner: parking_lot::RwLock<Option<endpoint::Endpoint>>,
    friend_request_receiver: Mutex<Option<mpsc::UnboundedReceiver<FriendRequest>>>,
    chat_request_receiver: Mutex<Option<mpsc::UnboundedReceiver<ChatRequest>>>,
    friend_request_next: parking_lot::Mutex<Option<FriendRequest>>,
    chat_request_next: parking_lot::Mutex<Option<ChatRequest>>,
    connections: parking_lot::RwLock<Slab<Connection>>,
}
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_secret_key() -> Result<Vec<u8>, Error> {
    Ok(SecretKey::generate(&mut rand::rng()).to_bytes().to_vec())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn get_secret_key_id(secret_key: Vec<u8>) -> Result<String, Error> {
    Ok(SecretKey::from_bytes(secret_key.as_slice().try_into()?)
        .public()
        .to_string())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_create(
    state: tauri::State<'_, State>,
    secret_key: Vec<u8>,
    person: Person,
) -> Result<(), Error> {
    let (friend_request_sender, friend_request_receiver) = mpsc::unbounded_channel();
    state
        .endpoint
        .friend_request_receiver
        .lock()
        .await
        .replace(friend_request_receiver);
    let (chat_request_sender, chat_request_receiver) = mpsc::unbounded_channel();
    state
        .endpoint
        .chat_request_receiver
        .lock()
        .await
        .replace(chat_request_receiver);
    let endpoint = endpoint::Endpoint::new(
        secret_key,
        person,
        friend_request_sender,
        chat_request_sender,
    )
    .await?;
    state.endpoint.inner.write().replace(endpoint);
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_is_create(state: tauri::State<'_, State>) -> Result<bool, Error> {
    Ok(state.endpoint.inner.read().is_some())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_request_person(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<Person, Error> {
    let endpoint = state.endpoint.inner.read().get_clone()?;
    Ok(endpoint.request_person(id.parse()?).await?)
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_request_friend(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<bool, Error> {
    let endpoint = state.endpoint.inner.read().get_clone()?;
    Ok(endpoint.request_friend(id.parse()?).await?)
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_request_chat(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<Option<usize>, Error> {
    let endpoint = state.endpoint.inner.read().get_clone()?;
    Ok(endpoint
        .request_chat(id.parse()?)
        .await?
        .map(|v| state.endpoint.connections.write().insert(v)))
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_friend_request_next(state: tauri::State<'_, State>) -> Result<bool, Error> {
    let friend_request_next = state
        .endpoint
        .friend_request_receiver
        .lock()
        .await
        .get_mut()?
        .recv()
        .await;
    let is_some = friend_request_next.is_some();
    *state.endpoint.friend_request_next.lock() = friend_request_next;
    Ok(is_some)
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_chat_request_next(state: tauri::State<'_, State>) -> Result<bool, Error> {
    let chat_request_next = state
        .endpoint
        .chat_request_receiver
        .lock()
        .await
        .get_mut()?
        .recv()
        .await;
    let is_some = chat_request_next.is_some();
    *state.endpoint.chat_request_next.lock() = chat_request_next;
    Ok(is_some)
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_connection_type(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<Option<String>, Error> {
    Ok(state
        .endpoint
        .inner
        .read()
        .get()?
        .connection_type(id.parse()?)
        .map(|value| {
            match value {
                ConnectionType::Direct(_) => "Direct",
                ConnectionType::Relay(_) => "Relay",
                ConnectionType::Mixed(_, _) => "Mixed",
                ConnectionType::None => "None",
            }
            .to_string()
        }))
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_latency(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<Option<Duration>, Error> {
    Ok(state.endpoint.inner.read().get()?.latency(id.parse()?))
}
