import { wrap, type Remote } from "comlink";
import type { FileSystemAdapter } from "./interface";
import { type OPFSFileSystem } from "opfs-worker";
import Worker from "opfs-worker/raw?worker";

export class WebFileSystem implements FileSystemAdapter {
  private worker: Worker;
  private opfs: Remote<OPFSFileSystem>;

  constructor() {
    const worker = new Worker();
    this.worker = worker;
    this.opfs = wrap<OPFSFileSystem>(worker);
  }
  free() {
    this.worker.terminate();
  }
  async remove_file(path: string) {
    await this.opfs.remove(path);
  }
}
