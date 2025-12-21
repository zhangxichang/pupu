import type { Instance } from "../interface";

export interface FileSystemAdapter extends Instance {
  remove_file(path: string): Promise<void>;
}
