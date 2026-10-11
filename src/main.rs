#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

#[cfg(not(target_family = "wasm"))]
#[tokio::main]
async fn main() -> eyre::Result<()> {
    use tokio::fs::create_dir_all;

    let data_path = directories::ProjectDirs::from("", "zhangxichang", "pupu")
        .ok_or_else(|| eyre::eyre!("无效应用目录"))?
        .data_dir()
        .to_path_buf();
    create_dir_all(&data_path).await?;
    app::main(Some(data_path)).await
}

#[cfg(all(target_os = "unknown", target_arch = "wasm32"))]
fn main() {
    wasm_bindgen_futures::spawn_local(async {
        app::main(None).await.unwrap();
    });
}
