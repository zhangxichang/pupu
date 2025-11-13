import type { OPFSFileSystem } from "opfs-worker";

type Native = { kind: "Native" } & typeof import("@tauri-apps/api/core");
type Web = { kind: "Web" } & typeof import("opfs-worker");

let api: Native | Web;
if (import.meta.env.TAURI_ENV_PLATFORM) {
  api = { kind: "Native", ...(await import("@tauri-apps/api/core")) };
}
if (!import.meta.env.TAURI_ENV_PLATFORM) {
  api = { kind: "Web", ...(await import("opfs-worker")) };
}

export class FileSystem {
  private opfs?: OPFSFileSystem;

  init() {
    if (api.kind === "Native") {
    } else if (api.kind === "Web") {
      if (this.opfs) return;
      this.opfs = api.createWorker();
    } else {
      throw new Error("API缺失");
    }
  }
  async remove_file(path: string) {
    if (api.kind === "Native") {
      try {
        await api.invoke("fs_remove_file", { path });
      } catch (error) {
        throw new Error(`${error}`);
      }
    } else if (api.kind === "Web") {
      if (!this.opfs) throw new Error("未初始化");
      await this.opfs.remove(path);
    } else {
      throw new Error("API缺失");
    }
  }
}
