import type { SQLiteAPI } from "@/worker/sqlite-api";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { CompiledQuery } from "kysely";

type Native = { kind: "Native" } & typeof import("@tauri-apps/api/core");
type Web = { kind: "Web" } & typeof import("@/worker/sqlite.ts?worker");

let api: Native | Web;
let event: typeof import("@tauri-apps/api/event");
if (import.meta.env.TAURI_ENV_PLATFORM) {
  api = { kind: "Native", ...(await import("@tauri-apps/api/core")) };
  event = await import("@tauri-apps/api/event");
}
let sqlite_api: typeof import("@/worker/sqlite-api");
if (!import.meta.env.TAURI_ENV_PLATFORM) {
  api = { kind: "Web", ...(await import("@/worker/sqlite?worker")) };
  sqlite_api = await import("@/worker/sqlite-api");
}

export interface SQLiteUpdateEvent {
  update_type: number;
  db_name: string | null;
  table_name: string | null;
  row_id: bigint;
}

export class Sqlite {
  private sqlite_api?: SQLiteAPI;
  private db?: number;
  private un_listen_on_update?: UnlistenFn;
  private schema_sql?: string;
  private on_updates = new Array<
    (event: SQLiteUpdateEvent) => void | Promise<void>
  >();

  async init() {
    if (api.kind === "Native") {
    } else if (api.kind === "Web") {
      if (this.sqlite_api) return;
      this.sqlite_api = await new Promise<SQLiteAPI>((resolve) => {
        const worker = new api.default();
        worker.onmessage = (e) => {
          resolve(new sqlite_api.SQLiteAPI(e.data));
          worker.onmessage = null;
        };
      });
    } else {
      throw new Error("API缺失");
    }
    if (this.schema_sql) return;
    this.schema_sql = await (await fetch("/schema.sql")).text();
  }
  async open(path: string, is_init?: boolean) {
    if (api.kind === "Native") {
      try {
        await api.invoke("sqlite_open", { path });
        await api.invoke("sqlite_on_update");
        this.unon_update = await event.listen<SQLiteUpdateEvent>(
          "on_update",
          async (e) => {
            for (const callback of this.on_updates) {
              await callback(e.payload);
            }
          },
        );
      } catch (error) {
        throw new Error(undefined, { cause: error });
      }
    } else if (api.kind === "Web") {
      if (!this.sqlite_api) throw new Error("未初始化");
      const db = await this.sqlite_api.open(path);
      this.sqlite_api.on_update(db, async (e) => {
        for (const callback of this.on_updates) {
          await callback(e);
        }
      });
      this.db = db;
    } else {
      throw new Error("API缺失");
    }
    if (is_init && this.schema_sql) {
      try {
        if (api.kind === "Native") {
          try {
            await api.invoke("sqlite_execute_batch", { sql: this.schema_sql });
          } catch (error) {
            throw new Error(undefined, { cause: error });
          }
        } else if (api.kind === "Web") {
          if (!this.sqlite_api) throw new Error("未初始化");
          if (!this.db) throw new Error("没有打开数据库");
          await this.sqlite_api.execute(this.db, this.schema_sql);
        } else {
          throw new Error("API缺失");
        }
      } catch {}
    }
  }
  async is_open() {
    if (api.kind === "Native") {
      try {
        return await api.invoke<boolean>("sqlite_is_open");
      } catch (error) {
        throw new Error(undefined, { cause: error });
      }
    } else if (api.kind === "Web") {
      return this.db ? true : false;
    } else {
      throw new Error("API缺失");
    }
  }
  async close() {
    if (api.kind === "Native") {
      try {
        await api.invoke("sqlite_close");
        this.un_listen_on_update?.();
      } catch (error) {
        throw new Error(undefined, { cause: error });
      }
    } else if (api.kind === "Web") {
      if (!this.sqlite_api) throw new Error("未初始化");
      if (!this.db) return;
      let db = this.db;
      this.db = undefined;
      await this.sqlite_api.close(db);
    } else {
      throw new Error("API缺失");
    }
  }
  async execute(compiled_query: CompiledQuery) {
    try {
      if (api.kind === "Native") {
        try {
          await api.invoke("sqlite_execute", {
            sql: compiled_query.sql,
            params: compiled_query.parameters,
          });
        } catch (error) {
          throw new Error(undefined, { cause: error });
        }
      } else if (api.kind === "Web") {
        if (!this.sqlite_api) throw new Error("未初始化");
        if (!this.db) throw new Error("没有打开数据库");
        await this.sqlite_api.execute(
          this.db,
          compiled_query.sql,
          compiled_query.parameters as any,
        );
      } else {
        throw new Error("API缺失");
      }
    } catch (error) {
      throw new Error(compiled_query.sql, { cause: error });
    }
  }
  async query<T>(compiled_query: CompiledQuery) {
    try {
      if (api.kind === "Native") {
        try {
          return await api.invoke<T[]>("sqlite_query", {
            sql: compiled_query.sql,
            params: compiled_query.parameters,
          });
        } catch (error) {
          throw new Error(undefined, { cause: error });
        }
      } else if (api.kind === "Web") {
        if (!this.sqlite_api) throw new Error("未初始化");
        if (!this.db) throw new Error("没有打开数据库");
        let result: T[] = [];
        for await (const value of await this.sqlite_api.query<T>(
          this.db,
          compiled_query.sql,
          compiled_query.parameters as any,
        )) {
          result.push(value);
        }
        return result;
      } else {
        throw new Error("API缺失");
      }
    } catch (error) {
      throw new Error(compiled_query.sql, { cause: error });
    }
  }
  on_update(callback: (event: SQLiteUpdateEvent) => void | Promise<void>) {
    this.on_updates.push(callback);
  }
  unon_update(callback: (event: SQLiteUpdateEvent) => void | Promise<void>) {
    this.on_updates = this.on_updates.filter((value) => value === callback);
  }
}
