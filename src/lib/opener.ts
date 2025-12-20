let opener: typeof import("@tauri-apps/plugin-opener") | undefined;
if (import.meta.env.TAURI_ENV_PLATFORM !== undefined) {
  opener = await import("@tauri-apps/plugin-opener");
}

export async function open_url(url: string) {
  if (opener) {
    await opener.openUrl(url);
  } else {
    open(url);
  }
}
