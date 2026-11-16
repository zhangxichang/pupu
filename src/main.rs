#![cfg(any(target_os = "windows", target_os = "linux"))]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use std::borrow::Cow;

use app::{AppConfig, paths::AppPaths};
use eyre::{Result, eyre};

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(tokio_runtime) => tokio_runtime.block_on(async {
            if let Err(err) = async_main().await {
                log::error!("程序运行出错：{}", err);
            }
        }),
        Err(err) => log::error!("tokio 运行时创建出错：{}", err),
    }
}

async fn async_main() -> Result<()> {
    let Some(dirs) = directories::ProjectDirs::from("com", "zhangxichang", "pupu") else {
        return Err(eyre!("应用路径初始化失败"));
    };
    let paths = AppPaths {
        data_dir: dirs.data_local_dir().to_path_buf(),
        config_dir: dirs.config_local_dir().to_path_buf(),
        cache_dir: dirs.cache_dir().to_path_buf(),
    };
    paths.build().await?;
    app::main(AppConfig {
        paths,
        font_data: Cow::Borrowed(include_bytes!("../assets/fonts/MiSans VF.ttf")),
    })
    .await?;
    Ok(())
}
