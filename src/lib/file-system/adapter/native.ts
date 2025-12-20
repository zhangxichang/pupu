import type { FileSystemAdapter } from "./interface";

export class NativeFileSystem implements FileSystemAdapter {
  async remove_file(path: string) {
    await new Promise((resolve) => {
      console.info(`原生实现的删除文件:${path}`);
      resolve(0);
    });
  }
}
