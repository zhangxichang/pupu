use std::path::PathBuf;

use eyre::Result;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_path: PathBuf,
    pub config_path: PathBuf,
    pub cache_path: PathBuf,
}
impl AppPaths {
    #[cfg(not(target_family = "wasm"))]
    pub async fn build(&self) -> Result<()> {
        use tokio::fs::create_dir_all;

        for path in [&self.data_path, &self.config_path, &self.cache_path] {
            create_dir_all(path).await?;
        }
        Ok(())
    }
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    pub async fn build(&self) -> Result<()> {
        todo!()
    }
}
