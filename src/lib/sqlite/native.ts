import { createTauRPCProxy, type JsonValue } from "~/generated/ipc_bindings";
import type { SQLiteAdapter } from "./interface";
import type { CompiledQuery } from "kysely";
import type { SQLiteUpdateEvent } from "./types";

export class NativeSQLite implements SQLiteAdapter {
  proxy: ReturnType<typeof createTauRPCProxy>["sqlite"];

  constructor() {
    this.proxy = createTauRPCProxy().sqlite;
  }
  init() {}
  free() {}
  async open(path: string) {
    await this.proxy.open(path);
  }
  async close() {
    await this.proxy.close();
  }
  async execute_sql(sql: string) {
    await this.proxy.execute_sql(sql);
  }
  async execute(compiled_query: CompiledQuery) {
    await this.proxy.execute(
      compiled_query.sql,
      compiled_query.parameters as JsonValue[],
    );
  }
  async query<T>(compiled_query: CompiledQuery) {
    return (await this.proxy.query(
      compiled_query.sql,
      compiled_query.parameters as JsonValue[],
    )) as T[];
  }
  async on_update(callback: (event: SQLiteUpdateEvent) => void) {
    await this.proxy.on_update(callback);
  }
}
