// #if TAURI_ENV_PLATFORM
import { invoke } from "@tauri-apps/api/core";
// #else
import {
  AsyncDuckDB,
  AsyncDuckDBConnection,
  ConsoleLogger,
  selectBundle,
} from "@duckdb/duckdb-wasm";
const { getJsDelivrBundles } = await import("@duckdb/duckdb-wasm");
// #endif

export class Database {
  // #if !TAURI_ENV_PLATFORM
  private duckdb?: AsyncDuckDB;
  private db?: AsyncDuckDBConnection;
  // #endif

  async init() {
    // #if TAURI_ENV_PLATFORM
    await invoke("state_database_init");
    // #else
    if (this.duckdb || this.db) return;
    const bundle = await selectBundle(getJsDelivrBundles());
    const worker_url = URL.createObjectURL(
      new Blob([`importScripts("${bundle.mainWorker}");`], {
        type: "text/javascript",
      }),
    );
    const duckdb = new AsyncDuckDB(new ConsoleLogger(), new Worker(worker_url));
    await duckdb.instantiate(bundle.mainModule, bundle.pthreadWorker);
    URL.revokeObjectURL(worker_url);
    this.duckdb = duckdb;
    this.db = await duckdb.connect();
    // #endif
  }
  async delete() {
    // #if TAURI_ENV_PLATFORM
    await invoke("state_database_delete");
    // #else
    if (!this.duckdb || !this.db) throw new Error("对象没有初始化");
    await this.db.close();
    await this.duckdb.dropFile("./data.db");
    // #endif
  }
}
