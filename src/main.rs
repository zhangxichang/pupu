#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

#[cfg(any(target_os = "windows", target_os = "linux"))]
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
            let Some(dirs) = directories::ProjectDirs::from("com", "zhangxichang", "pupu") else {
                log::error!("应用路径初始化失败");
                return;
            };
            let paths = app::paths::AppPaths {
                data_dir: dirs.data_local_dir().to_path_buf(),
                config_dir: dirs.config_local_dir().to_path_buf(),
                cache_dir: dirs.cache_dir().to_path_buf(),
            };
            if let Err(err) = paths.build().await {
                log::error!("应用路径构建出错：{}", err);
                return;
            }
            if let Err(err) = app::main(paths).await {
                log::error!("应用运行出错：{}", err);
                return;
            }
        }),
        Err(err) => log::error!("tokio 运行时创建出错：{}", err),
    }
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).expect("error initializing logger");

    wasm_bindgen_futures::spawn_local(async {
        let paths = app::paths::AppPaths {
            data_dir: "data".into(),
            config_dir: "config".into(),
            cache_dir: "cache".into(),
        };
        if let Err(err) = paths.build().await {
            log::error!("应用路径构建出错：{}", err);
            return;
        }
        if let Err(err) = app::main(paths).await {
            log::error!("应用运行出错：{}", err);
            return;
        }
    });
}
