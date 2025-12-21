import type { Instance } from "../interface";

export interface SQLiteAdapter extends Instance {
  test(): Promise<void>;
}
