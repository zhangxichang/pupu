import { invoke } from "@tauri-apps/api/core";

export async function remove_file(path: string) {
  try {
    await invoke("fs_remove_file", { path });
  } catch (error) {
    throw new Error(String(error));
  }
}
export async function read_file(path: string) {
  try {
    return Uint8Array.from(await invoke("fs_read_file", { path }));
  } catch (error) {
    throw new Error(String(error));
  }
}
export async function create_file(path: string, bytes: Uint8Array) {
  try {
    await invoke("fs_create_file", { path, bytes });
  } catch (error) {
    throw new Error(String(error));
  }
}
export async function exists(path: string) {
  try {
    return await invoke<boolean>("fs_exists", { path });
  } catch (error) {
    throw new Error(String(error));
  }
}
export async function remove_dir_all(path: string) {
  try {
    await invoke("fs_remove_dir_all", { path });
  } catch (error) {
    throw new Error(String(error));
  }
}
