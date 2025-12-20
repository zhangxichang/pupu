import type { FileSystemAdapter } from "./interface";
import { createWorker, type OPFSFileSystem } from "opfs-worker";

export class WebFileSystem implements FileSystemAdapter {
  private opfs: OPFSFileSystem;

  private constructor(opfs: OPFSFileSystem) {
    this.opfs = opfs;
  }
  static new() {
    return new WebFileSystem(createWorker());
  }
  async remove_file(path: string) {
    console.info(`Web实现的删除文件:${path}`);
    await this.opfs.remove(path);
  }
}
