pub fn tauri_plugin_builder<R: tauri::Runtime>(
    #[allow(unused_mut)] mut builder: tauri::Builder<R>,
) -> tauri::Builder<R> {
    #[cfg(all(desktop, not(debug_assertions)))]
    {
        use tauri::Manager;

        builder = builder.plugin(tauri_plugin_single_instance::init(
            |tauri_app, _args, _cwd| {
                tauri_app
                    .get_webview_window("main")
                    .unwrap()
                    .set_focus()
                    .unwrap();
            },
        ));
    }
    #[allow(unused_mut)]
    let mut tauri_plugin_prevent_default = tauri_plugin_prevent_default::Builder::new();
    #[cfg(target_os = "windows")]
    {
        tauri_plugin_prevent_default =
            tauri_plugin_prevent_default.platform(tauri_plugin_prevent_default::PlatformOptions {
                general_autofill: false,
                password_autosave: false,
            });
    }
    builder
        .plugin(tauri_plugin_prevent_default.build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
}
