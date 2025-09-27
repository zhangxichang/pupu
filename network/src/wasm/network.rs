pub mod user_protocol;

use wasm_bindgen::{JsError, prelude::wasm_bindgen};

use crate::{
    network,
    wasm::{network::user_protocol::Account, utils::MapJsError},
};

#[wasm_bindgen]
pub struct User(protocol::User);
#[wasm_bindgen]
impl User {
    pub fn new(id: String, name: String) -> Self {
        Self(protocol::User { id, name })
    }
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.0.name.clone()
    }
}
impl From<User> for protocol::User {
    fn from(value: User) -> Self {
        value.0
    }
}
impl From<protocol::User> for User {
    fn from(value: protocol::User) -> Self {
        Self(value)
    }
}

#[wasm_bindgen]
pub struct Network(network::Network);
#[wasm_bindgen]
impl Network {
    pub async fn new(account: Account) -> Result<Self, JsError> {
        Ok(Self(network::Network::new(account.into()).await.mje()?))
    }
    pub fn account(&self) -> Account {
        (*self.0.account()).clone().into()
    }
    pub async fn search_user(&self, user_id: String) -> Result<User, JsError> {
        Ok(self.0.search_user(user_id).await.mje()?.into())
    }
}
