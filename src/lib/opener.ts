// #if TAURI_ENV_PLATFORM
import { openUrl } from "@tauri-apps/plugin-opener";
// #endif

export async function open_url(url: string) {
  // #if TAURI_ENV_PLATFORM
  await openUrl(url);
  // #else
  open(url);
  // #endif
}
