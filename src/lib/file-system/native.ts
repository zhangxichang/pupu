import { invoke } from "@tauri-apps/api/core";
import type { FileSystemAdapter } from "./interface";

export class NativeFileSystem implements FileSystemAdapter {
  free() {}
  async remove_file(path: string) {
    await invoke("remove_file", { path });
  }
}
