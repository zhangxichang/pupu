mod migration;
#[cfg(not(target_family = "wasm"))]
mod native;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub mod web;

use std::path::PathBuf;

use eyre::Result;

pub trait Storage {}

pub fn open(data_path: PathBuf) -> Result<impl Storage> {
    #[cfg(not(target_family = "wasm"))]
    {
        use crate::storage::native::NativeStorage;

        NativeStorage::open(data_path)
    }
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    {
        use crate::storage::web::client::WebStorage;

        Ok(WebStorage::open(data_path))
    }
}
