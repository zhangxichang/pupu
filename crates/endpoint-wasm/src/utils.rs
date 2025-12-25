use base64::{Engine, prelude::BASE64_STANDARD};
use iroh_gossip::TopicId;
use wasm_bindgen::{JsError, prelude::wasm_bindgen};

use crate::types::Ticket;

#[wasm_bindgen]
pub fn generate_secret_key() -> Vec<u8> {
    iroh::SecretKey::generate(&mut rand::rng())
        .to_bytes()
        .to_vec()
}
#[wasm_bindgen]
pub fn get_secret_key_id(secret_key: Vec<u8>) -> Result<String, JsError> {
    Ok(
        iroh::SecretKey::from_bytes(secret_key.as_slice().try_into()?)
            .public()
            .to_string(),
    )
}
#[wasm_bindgen]
pub fn generate_group_id() -> String {
    TopicId::from_bytes(rand::random()).to_string()
}
#[wasm_bindgen]
pub fn generate_ticket(group_id: String, bootstrap: Vec<String>) -> Result<String, JsError> {
    Ok(BASE64_STANDARD.encode(serde_json::to_vec(&Ticket {
        id: group_id.parse()?,
        bootstrap: bootstrap
            .into_iter()
            .map(|v| v.parse())
            .collect::<Result<_, _>>()?,
    })?))
}
