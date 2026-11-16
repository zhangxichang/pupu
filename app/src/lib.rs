#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    all(target_family = "wasm", target_os = "unknown"),
)))]
compile_error!("Unsupported target platform");

mod contributors;
mod mock_data;
pub mod paths;
mod virtual_list_layout;
mod window_resize;

use std::{borrow::Cow, sync::Arc};

use eyre::Result;
use file_picker::FilePickerBackend;
use slint::{Model, fontique_012};

use crate::paths::AppPaths;

slint::include_modules!();

impl From<PickOptions> for file_picker::PickOptions {
    fn from(options: PickOptions) -> Self {
        file_picker::PickOptions {
            title: options.title.to_string(),
            filters: options
                .filters
                .iter()
                .map(|filter| file_picker::FileFilter {
                    name: filter.name.to_string(),
                    extensions: filter
                        .extensions
                        .iter()
                        .map(|ext| ext.to_string())
                        .collect(),
                })
                .collect(),
            mime_types: options
                .mime_types
                .iter()
                .map(|mime| mime.to_string())
                .collect(),
        }
    }
}

pub struct AppConfig {
    pub paths: AppPaths,
    pub font_data: Cow<'static, [u8]>,
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    pub storage_worker_url: String,
}

pub async fn main(config: AppConfig) -> Result<()> {
    setup_backend()?;
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
    let storage = storage::native::Storage::open(config.paths.data_dir).await?;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    let storage =
        storage::web::Storage::open(config.storage_worker_url, config.paths.data_dir).await?;
    setup_font(config.font_data)?;
    setup_viewport()?.run()?;
    drop(storage);
    Ok(())
}

fn setup_backend() -> Result<()> {
    let backend = slint::BackendSelector::new();
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let backend = backend
        .backend_name("winit".to_string())
        .renderer_name("skia".to_string())
        .with_winit_window_attributes_hook(|attributes| attributes.with_transparent(true));
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
    let backend = {
        use slint::wgpu_30::{WGPUConfiguration, WGPUSettings, wgpu};

        let mut settings = WGPUSettings::default();
        #[cfg(target_os = "windows")]
        {
            settings.backends = wgpu::Backends::DX12;
            settings.backend_options.dx12.presentation_system =
                wgpu::Dx12SwapchainKind::DxgiFromVisual;
        }
        #[cfg(target_os = "linux")]
        {
            settings.backends = wgpu::Backends::VULKAN;
        }
        #[cfg(target_os = "android")]
        {
            settings.backends = wgpu::Backends::VULKAN;
            settings.power_preference = wgpu::PowerPreference::LowPower;
        }
        backend.require_wgpu_30(WGPUConfiguration::Automatic(settings))
    };
    backend.select()?;
    Ok(())
}

fn setup_font(font_data: Cow<'static, [u8]>) -> Result<()> {
    let blob = match font_data {
        Cow::Borrowed(data) => fontique_012::fontique::Blob::new(Arc::new(data)),
        Cow::Owned(data) => fontique_012::fontique::Blob::new(Arc::new(data)),
    };
    if fontique_012::shared_collection()
        .register_fonts(blob, None)
        .is_empty()
    {
        return Err(eyre::eyre!("字体注册失败"));
    }
    Ok(())
}

fn setup_viewport() -> Result<Viewport> {
    let viewport = Viewport::new()?;
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    viewport.set_platform(AppPlatform::Desktop);
    #[cfg(target_os = "android")]
    viewport.set_platform(AppPlatform::Mobile);
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    viewport.set_platform(AppPlatform::Web);
    #[cfg(any(target_os = "android", target_os = "windows", target_os = "linux"))]
    viewport.set_app_version(env!("APP_VERSION").into());
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use slint::{platform::WindowEvent, winit_030::WinitWindowAccessor};

        viewport.on_drag_window({
            let viewport = viewport.as_weak();
            move || {
                if let Some(viewport) = viewport.upgrade() {
                    _ = viewport
                        .window()
                        .with_winit_window(|window| window.drag_window());
                }
                _ = viewport.upgrade_in_event_loop(|viewport| {
                    _ = viewport
                        .window()
                        .dispatch_event_with_result(WindowEvent::PointerExited);
                });
            }
        });
        viewport.on_resize_window({
            let viewport = viewport.as_weak();
            move |direction| {
                if let Some(viewport) = viewport.upgrade() {
                    _ = viewport
                        .window()
                        .with_winit_window(|window| window.drag_resize_window(direction.into()));
                }
                _ = viewport.upgrade_in_event_loop(|viewport| {
                    _ = viewport
                        .window()
                        .dispatch_event_with_result(WindowEvent::PointerExited);
                });
            }
        });
    }
    #[cfg(any(target_os = "android", target_os = "windows", target_os = "linux"))]
    viewport.on_get_latest_version(|| "0.2.0".into());
    #[cfg(any(target_os = "android", target_os = "windows", target_os = "linux"))]
    viewport.on_compare_versions(|a, b| {
        let ordering = match (
            semver::Version::parse(a.as_str()),
            semver::Version::parse(b.as_str()),
        ) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            _ => std::cmp::Ordering::Equal,
        };
        match ordering {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    });
    viewport.on_open_url(|url| {
        if let Err(err) = webbrowser::open(url.as_str()) {
            log::warn!("打开链接失败：{err}");
        }
    });
    viewport.on_pick_register_avatar({
        let viewport = viewport.as_weak();
        move |options| {
            let viewport = viewport.clone();
            let options = options.into();
            let task = async move {
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                let file = file_picker::desktop::PickedFile::new(options).await;
                #[cfg(target_os = "android")]
                let file = file_picker::android::PickedFile::new(options).await;
                #[cfg(all(target_family = "wasm", target_os = "unknown"))]
                let file = file_picker::web::PickedFile::new(options).await;
                let file = match file {
                    Ok(Some(file)) => file,
                    Ok(None) => return,
                    Err(err) => {
                        log::warn!("选择文件失败：{err:?}");
                        return;
                    }
                };
                let bytes = match file.read_to_end().await {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        log::warn!("读取文件失败：{err:?}");
                        return;
                    }
                };
                let Ok(image) = image::load_from_memory(&bytes) else {
                    log::warn!("头像图片解码失败");
                    return;
                };
                let image = image.to_rgba8();
                let buffer = slint::SharedPixelBuffer::clone_from_slice(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                );
                _ = slint::invoke_from_event_loop(move || {
                    if let Some(viewport) = viewport.upgrade() {
                        viewport.set_register_avatar_image(slint::Image::from_rgba8(buffer));
                        viewport.set_register_has_avatar(true);
                    }
                });
            };
            #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
            tokio::spawn(task);
            #[cfg(all(target_family = "wasm", target_os = "unknown"))]
            wasm_bindgen_futures::spawn_local(task);
        }
    });
    virtual_list_layout::setup(&viewport);
    contributors::start(viewport.as_weak());
    mock_data::setup(&viewport);
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    {
        use slint::RenderingState;

        let mut first_frame = true;
        viewport.window().set_rendering_notifier(move |state, _| {
            if !first_frame || !matches!(state, RenderingState::AfterRendering) {
                return;
            }
            first_frame = false;
            ready_notify::notify_ready();
        })?;
    }
    Ok(viewport)
}
