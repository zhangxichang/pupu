#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    let Some(data_dir) = app.internal_data_path() else {
        eprintln!("Android internal data path is unavailable");
        return;
    };
    let Some(app_root) = data_dir.parent().map(|path| path.to_path_buf()) else {
        eprintln!("Android app data root is unavailable");
        return;
    };

    let paths = app::AppPaths {
        config_dir: data_dir.join("config"),
        cache_dir: app_root.join("cache"),
        data_dir,
    };

    slint::android::init(app).unwrap();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            app::main(Some(paths)).await.unwrap();
        });
}
