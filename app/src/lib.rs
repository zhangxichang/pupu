#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    all(target_family = "wasm", target_os = "unknown"),
)))]
compile_error!("Unsupported target platform");

pub mod paths;
mod window_resize;

use std::{borrow::Cow, sync::Arc};

use eyre::Result;
use slint::fontique_011;

use crate::paths::AppPaths;

slint::include_modules!();

pub struct AppConfig {
    pub paths: AppPaths,
    pub font_data: Cow<'static, [u8]>,
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    pub storage_worker_url: String,
}

pub async fn main(config: AppConfig) -> Result<()> {
    let backend = slint::BackendSelector::new();
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let backend = backend
        .backend_name("winit".to_string())
        .renderer_name("femtovg-wgpu".to_string())
        .with_winit_window_attributes_hook(|attributes| attributes.with_transparent(true));
    #[cfg(target_os = "windows")]
    let backend = {
        use slint::wgpu_30::{WGPUConfiguration, WGPUSettings, wgpu};

        let mut settings = WGPUSettings::default();
        settings.backends = wgpu::Backends::DX12;
        settings.backend_options.dx12.presentation_system = wgpu::Dx12SwapchainKind::DxgiFromVisual;
        backend.require_wgpu_30(WGPUConfiguration::Automatic(settings))
    };
    #[cfg(target_os = "android")]
    let backend = {
        use slint::wgpu_30::WGPUConfiguration;

        backend.require_wgpu_30(WGPUConfiguration::default())
    };
    backend.select()?;
    let blob = match config.font_data {
        Cow::Borrowed(data) => fontique_011::fontique::Blob::new(Arc::new(data)),
        Cow::Owned(data) => fontique_011::fontique::Blob::new(Arc::new(data)),
    };
    if fontique_011::shared_collection()
        .register_fonts(blob, None)
        .is_empty()
    {
        return Err(eyre::eyre!("字体注册失败"));
    }
    let viewport = Viewport::new()?;
    viewport.set_enabled_window_chrome(cfg!(any(target_os = "windows", target_os = "linux")));
    viewport
        .set_enabled_update_notification(cfg!(any(target_os = "windows", target_os = "linux",)));
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
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    {
        use slint::RenderingState;

        let mut first_frame = true;
        viewport.window().set_rendering_notifier(move |state, _| {
            if !first_frame || !matches!(state, RenderingState::AfterRendering) {
                return;
            }
            first_frame = false;
            lifecycle::notify_ready();
        })?;
    }
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "android"))]
    let storage = storage::open(config.paths.data_dir).await?;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    let storage = storage::open(config.paths.data_dir, &config.storage_worker_url).await?;
    viewport.run()?;
    drop(storage);
    Ok(())
}
