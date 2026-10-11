mod migration;
#[cfg(not(target_family = "wasm"))]
mod native;
#[cfg(all(target_os = "unknown", target_arch = "wasm32"))]
pub mod web;

use std::path::PathBuf;

use eyre::Result;

pub trait Storage {}

#[cfg(not(target_family = "wasm"))]
pub fn open(data_path: Option<PathBuf>) -> Result<impl Storage> {
    use crate::storage::native::NativeStorage;

    NativeStorage::open(data_path.ok_or_else(|| eyre::eyre!("原生平台必须提供数据目录"))?)
}

#[cfg(all(target_os = "unknown", target_arch = "wasm32"))]
pub fn open(_: Option<PathBuf>) -> Result<impl Storage> {
    use crate::storage::web::client::WebStorage;

    Ok(WebStorage::new())
}
