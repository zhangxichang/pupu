mod native;
pub mod web;

use std::path::PathBuf;

use eyre::Result;

pub trait Storage {}

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
pub async fn open(data_dir: PathBuf) -> Result<impl Storage> {
    use crate::native::NativeStorage;

    NativeStorage::open(data_dir)
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub async fn open(data_dir: PathBuf, worker_url: &str) -> Result<impl Storage> {
    use crate::web::WebStorage;

    WebStorage::open(data_dir, worker_url).await
}
