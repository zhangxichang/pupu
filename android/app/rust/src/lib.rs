#![cfg(target_os = "android")]

mod jni_bridge;

use std::borrow::Cow;

use app::{AppConfig, paths::AppPaths};
use eyre::{Result, eyre};
use slint::android::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(tokio_runtime) => tokio_runtime.block_on(async move {
            if let Err(err) = async_main(app).await {
                log::error!("程序运行出错：{}", err);
            }
        }),
        Err(err) => log::error!("tokio 运行时创建出错：{}", err),
    }
}

async fn async_main(app: AndroidApp) -> Result<()> {
    let Some(data_dir) = app.internal_data_path() else {
        return Err(eyre!("应用数据路径不存在"));
    };
    let Some(app_root_path) = data_dir.parent().map(|path| path.to_path_buf()) else {
        return Err(eyre!("应用根路径不存在"));
    };
    let paths = AppPaths {
        config_dir: data_dir.clone(),
        cache_dir: app_root_path.join("cache"),
        data_dir,
    };
    paths.build().await?;
    slint::android::init(app)?;
    app::main(AppConfig {
        paths,
        font_data: Cow::Borrowed(include_bytes!("../../../../assets/fonts/MiSans VF.ttf")),
    })
    .await?;
    Ok(())
}
