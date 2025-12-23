import type { CompiledQuery } from "kysely";
import type { Instance } from "../interface";

export interface SQLiteAdapter extends Instance {
  open(path: string): Promise<void>;
  close(): Promise<void>;
  execute(compiled_query: CompiledQuery): Promise<void>;
  query<T>(compiled_query: CompiledQuery): Promise<T[]>;
  on_update(
    callback: (
      update_type: number,
      db_name: string | null,
      table_name: string | null,
      row_id: bigint,
    ) => void,
  ): void;
}
