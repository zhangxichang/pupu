pub mod connection;
pub mod person_protocol_event_next;

use iroh::{
    SecretKey, Watcher,
    endpoint::{Connection, ConnectionType},
    protocol::Router,
};
use iroh_blobs::{BlobsProtocol, store::fs::FsStore};
use iroh_gossip::Gossip;
use parking_lot::RwLock;
use person_protocol::{Person, PersonProtocol};
use slab::Slab;
use tokio::sync::mpsc;

use crate::{
    error::{Error, OptionGet, OptionGetClone},
    state::State,
};

#[derive(Default)]
pub struct Endpoint {
    router: RwLock<Option<Router>>,
    person_protocol: RwLock<Option<PersonProtocol>>,
    person_protocol_event_receiver:
        tokio::sync::Mutex<Option<mpsc::UnboundedReceiver<person_protocol::Event>>>,
    person_protocol_event_next: parking_lot::Mutex<Option<person_protocol::Event>>,
    gossip_protocol: RwLock<Option<Gossip>>,
    blobs_protocol: RwLock<Option<BlobsProtocol>>,
    connections: RwLock<Slab<Connection>>,
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_generate_secret_key() -> Result<Vec<u8>, Error> {
    Ok(SecretKey::generate(&mut rand::rng()).to_bytes().to_vec())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_get_secret_key_id(secret_key: Vec<u8>) -> Result<String, Error> {
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
    let (person_protocol_event_sender, person_protocol_event_receiver) = mpsc::unbounded_channel();
    let endpoint = iroh::Endpoint::builder()
        .secret_key(SecretKey::from_bytes(secret_key.as_slice().try_into()?))
        .bind()
        .await?;
    let person_protocol =
        PersonProtocol::new(endpoint.clone(), person, person_protocol_event_sender);
    state
        .endpoint
        .person_protocol_event_receiver
        .lock()
        .await
        .replace(person_protocol_event_receiver);
    state
        .endpoint
        .person_protocol
        .write()
        .replace(person_protocol.clone());
    let gossip_protocol = Gossip::builder().spawn(endpoint.clone());
    state
        .endpoint
        .gossip_protocol
        .write()
        .replace(gossip_protocol.clone());
    let blobs_protocol = BlobsProtocol::new(
        &FsStore::load("store")
            .await
            .map_err(|err| Error::from(err.to_string()))?
            .into(),
        None,
    );
    state
        .endpoint
        .blobs_protocol
        .write()
        .replace(blobs_protocol.clone());
    state.endpoint.router.write().replace(
        Router::builder(endpoint)
            .accept(person_protocol::ALPN, person_protocol)
            .accept(iroh_gossip::ALPN, gossip_protocol)
            .accept(iroh_blobs::ALPN, blobs_protocol)
            .spawn(),
    );
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_is_create(state: tauri::State<'_, State>) -> Result<bool, Error> {
    Ok(state.endpoint.router.read().is_some())
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_person_protocol_event_next(
    state: tauri::State<'_, State>,
) -> Result<Option<String>, Error> {
    let next = state
        .endpoint
        .person_protocol_event_receiver
        .lock()
        .await
        .get_mut()?
        .recv()
        .await;
    let kind = next.as_ref().map(|v| v.to_string());
    *state.endpoint.person_protocol_event_next.lock() = next;
    Ok(kind)
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_request_person(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<Person, Error> {
    let endpoint = state.endpoint.person_protocol.read().get_clone()?;
    Ok(endpoint.request_person(id.parse()?).await?)
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_request_friend(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<bool, Error> {
    let endpoint = state.endpoint.person_protocol.read().get_clone()?;
    Ok(endpoint.request_friend(id.parse()?).await?)
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_request_chat(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<Option<usize>, Error> {
    let endpoint = state.endpoint.person_protocol.read().get_clone()?;
    Ok(endpoint
        .request_chat(id.parse()?)
        .await?
        .map(|v| state.endpoint.connections.write().insert(v)))
}
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_conn_type(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<Option<String>, Error> {
    Ok(state
        .endpoint
        .router
        .read()
        .get()?
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
#[tauri::command(rename_all = "snake_case")]
pub async fn endpoint_latency(
    state: tauri::State<'_, State>,
    id: String,
) -> Result<Option<u128>, Error> {
    Ok(state
        .endpoint
        .router
        .read()
        .get()?
        .endpoint()
        .latency(id.parse()?)
        .map(|v| v.as_millis()))
}
