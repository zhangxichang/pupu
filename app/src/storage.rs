mod migration;
#[cfg(not(target_family = "wasm"))]
mod native;
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub mod web;

use std::path::PathBuf;

use eyre::Result;

pub trait Storage {}

#[cfg(not(target_family = "wasm"))]
pub fn open(data_path: Option<PathBuf>) -> Result<impl Storage> {
    use eyre::ContextCompat;

    use crate::storage::native::NativeStorage;

    NativeStorage::open(data_path.wrap_err("原生平台必须提供数据目录")?)
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub fn open(_: Option<PathBuf>) -> Result<impl Storage> {
    use crate::storage::web::client::WebStorage;

    Ok(WebStorage::new())
}
