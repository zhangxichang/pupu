import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

export async function open_url(url: string) {
  if (isTauri()) {
    await openUrl(url);
  } else {
    open(url);
  }
}
