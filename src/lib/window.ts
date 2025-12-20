let window: typeof import("@tauri-apps/api/window") | undefined;
if (import.meta.env.TAURI_ENV_PLATFORM !== undefined) {
  window = await import("@tauri-apps/api/window");
}

export function get_window() {
  if (window) {
    return window.getCurrentWindow();
  }
}
