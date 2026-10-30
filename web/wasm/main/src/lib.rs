#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use app::{AppConfig, paths::AppPaths};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn main(font_data: Vec<u8>, storage_worker_url: String) -> Result<(), JsError> {
    console_error_panic_hook::set_once();
    if let Err(err) = console_log::init_with_level(log::Level::Info) {
        log::warn!("日志初始化失败：{}", err);
    }
    wasm_bindgen_futures::spawn_local(async {
        let paths = AppPaths {
            data_dir: "data".into(),
            config_dir: "config".into(),
            cache_dir: "cache".into(),
        };
        if let Err(err) = paths.build().await {
            lifecycle::notify_error(format!("{}", err));
            return;
        }
        if let Err(err) = app::main(AppConfig {
            paths,
            font_data: font_data.into(),
            storage_worker_url,
        })
        .await
        {
            lifecycle::notify_error(format!("{}", err));
            return;
        }
    });
    lifecycle::wait().await
}
