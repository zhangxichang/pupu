import { invoke } from "@tauri-apps/api/core";

export async function fatal_error(error: Error) {
  try {
    await invoke("fatal_error", {
      stack: error.stack,
    });
  } catch (error) {
    throw new Error(String(error));
  }
}
