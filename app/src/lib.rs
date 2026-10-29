#[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "android",
    all(target_family = "wasm", target_os = "unknown"),
)))]
compile_error!("Unsupported target platform");

pub mod paths;
pub mod storage;
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod window_resize;

use eyre::Result;

use crate::paths::AppPaths;

slint::include_modules!();

pub async fn main(paths: AppPaths) -> Result<()> {
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
    let storage = storage::open(paths.data_dir)?;
    viewport.run()?;
    drop(storage);
    Ok(())
}
