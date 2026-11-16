#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use app::{AppConfig, paths::AppPaths};
use eyre::Result;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn main(font_data: Vec<u8>, storage_worker_url: String) -> Result<(), JsError> {
    console_error_panic_hook::set_once();
    if let Err(err) = console_log::init_with_level(log::Level::Info) {
        log::warn!("日志初始化失败：{}", err);
    }
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(err) = async_main(font_data, storage_worker_url).await {
            log::error!("程序运行出错：{}", err);
            ready_notify::notify_error(format!("程序运行出错：{}", err));
        }
    });
    ready_notify::wait().await
}

async fn async_main(font_data: Vec<u8>, storage_worker_url: String) -> Result<()> {
    let paths = AppPaths {
        data_dir: "data".into(),
        config_dir: "config".into(),
        cache_dir: "cache".into(),
    };
    paths.build().await?;
    app::main(AppConfig {
        paths,
        font_data: font_data.into(),
        storage_worker_url,
    })
    .await?;
    Ok(())
}
