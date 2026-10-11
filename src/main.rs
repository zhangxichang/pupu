#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

#[cfg(any(target_os = "windows", target_os = "linux"))]
#[tokio::main]
async fn main() -> eyre::Result<()> {
    use eyre::ContextCompat;

    let dirs =
        directories::ProjectDirs::from("com", "zhangxichang", "pupu").wrap_err("目录获取失败")?;
    let paths = app::AppPaths {
        data_dir: dirs.data_local_dir().to_path_buf(),
        config_dir: dirs.config_dir().to_path_buf(),
        cache_dir: dirs.cache_dir().to_path_buf(),
    };
    app::main(Some(paths)).await?;
    Ok(())
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn main() {
    wasm_bindgen_futures::spawn_local(async {
        app::main(None).await.unwrap();
    });
}
