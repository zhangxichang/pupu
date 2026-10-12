#[cfg(any(target_os = "windows", target_os = "linux"))]
mod window_resize;

use std::path::PathBuf;

use eyre::Result;

slint::include_modules!();

pub async fn main(_data_path: Option<PathBuf>) -> Result<()> {
    let backend = slint::BackendSelector::new();
    #[cfg(not(any(target_family = "wasm", target_os = "android")))]
    let backend = backend.backend_name("winit".to_string());
    #[cfg(not(any(target_family = "wasm", target_os = "android")))]
    let backend = backend.renderer_name("femtovg-wgpu".to_string());
    #[cfg(target_os = "windows")]
    let backend = {
        use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;

        backend.with_winit_window_attributes_hook(|attributes| {
            attributes.with_undecorated_shadow(true)
        })
    };
    #[cfg(target_os = "linux")]
    let backend =
        backend.with_winit_window_attributes_hook(|attributes| attributes.with_transparent(true));
    #[cfg(target_os = "android")]
    let backend = {
        use slint::wgpu_29::{WGPUConfiguration, WGPUSettings, wgpu::InstanceFlags};

        let mut wgpu_settings = WGPUSettings::default();
        wgpu_settings.instance_flags.remove(InstanceFlags::DEBUG);
        backend.require_wgpu_29(WGPUConfiguration::Automatic(wgpu_settings))
    };
    backend.select()?;
    let viewport = Viewport::new()?;
    viewport.set_enabled_window_chrome(cfg!(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "macos"
    )));
    viewport.set_enabled_resize_handles(cfg!(any(target_os = "windows", target_os = "linux")));
    viewport.set_enabled_linux_window_shadow(cfg!(target_os = "linux"));
    viewport.set_enabled_update_notification(cfg!(not(target_family = "wasm")));
    #[cfg(target_os = "windows")]
    viewport.set_chrome_insets(Insets {
        bottom: 1.,
        left: 1.,
        right: 1.,
        top: 0.,
    });
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        use slint::winit_030::WinitWindowAccessor;

        let viewport_weak = viewport.as_weak();
        viewport.on_drag_window(move || {
            if let Some(viewport) = viewport_weak.upgrade() {
                _ = viewport
                    .window()
                    .with_winit_window(|window| window.drag_window());
            }
            _ = viewport_weak.upgrade_in_event_loop(|viewport| {
                _ = viewport
                    .window()
                    .dispatch_event_with_result(slint::platform::WindowEvent::PointerExited);
            });
        });
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use slint::winit_030::WinitWindowAccessor;

        let viewport_weak = viewport.as_weak();
        viewport.on_resize_window(move |direction| {
            if let Some(viewport) = viewport_weak.upgrade() {
                _ = viewport
                    .window()
                    .with_winit_window(|window| window.drag_resize_window(direction.into()));
            }
            _ = viewport_weak.upgrade_in_event_loop(|viewport| {
                _ = viewport
                    .window()
                    .dispatch_event_with_result(slint::platform::WindowEvent::PointerExited);
            });
        });
    }
    viewport.run()?;
    Ok(())
}
