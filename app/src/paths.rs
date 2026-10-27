use std::path::PathBuf;

use eyre::Result;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}
impl AppPaths {
    #[cfg(not(target_family = "wasm"))]
    pub async fn build(&self) -> Result<()> {
        use tokio::fs::create_dir_all;

        for path in [&self.data_dir, &self.config_dir, &self.cache_dir] {
            create_dir_all(path).await?;
        }
        Ok(())
    }
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    pub async fn build(&self) -> Result<()> {
        use std::path::Component;

        use opfs::persistent::app_specific_dir;
        use opfs::{DirectoryHandle, GetDirectoryHandleOptions};

        for path in [&self.data_dir, &self.config_dir, &self.cache_dir] {
            let mut dir = app_specific_dir()
                .await
                .map_err(|err| eyre::eyre!("{:?}", err))?;

            for component in path.components() {
                let Component::Normal(name) = component else {
                    continue;
                };
                let name = name
                    .to_str()
                    .ok_or_else(|| eyre::eyre!("invalid OPFS path component"))?;
                dir = dir
                    .get_directory_handle_with_options(
                        name,
                        &GetDirectoryHandleOptions { create: true },
                    )
                    .await
                    .map_err(|err| eyre::eyre!("{:?}", err))?;
            }
        }
        Ok(())
    }
}
