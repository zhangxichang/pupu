import { invoke } from "@tauri-apps/api/core";

export async function log_error(error: string) {
  try {
    await invoke("log_error", { error });
  } catch (error) {
    throw new Error(String(error));
  }
}
