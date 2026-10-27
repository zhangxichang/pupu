#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(tokio_runtime) => tokio_runtime.block_on(async {
            let Some(data_dir) = app.internal_data_path() else {
                log::error!("应用数据路径不存在");
                return;
            };
            let Some(app_root_path) = data_dir.parent().map(|path| path.to_path_buf()) else {
                log::error!("应用根路径不存在");
                return;
            };
            let paths = app::paths::AppPaths {
                config_dir: data_dir.clone(),
                cache_dir: app_root_path.join("cache"),
                data_dir,
            };
            if let Err(err) = paths.build().await {
                log::error!("应用路径构建出错：{}", err);
                return;
            }
            if let Err(err) = slint::android::init(app) {
                log::error!("应用初始化出错：{}", err);
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
