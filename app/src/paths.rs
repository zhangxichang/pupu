use std::path::PathBuf;

use eyre::Result;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}
impl AppPaths {
    pub async fn build(&self) -> Result<()> {
        for path in [&self.data_dir, &self.config_dir, &self.cache_dir] {
            #[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
            tokio::fs::create_dir_all(path).await?;
            #[cfg(all(target_family = "wasm", target_os = "unknown"))]
            tokio_fs_ext::create_dir_all(path).await?;
        }
        Ok(())
    }
}
