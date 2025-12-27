import { proxy, wrap, type Remote } from "comlink";
import type { SQLite, SQLiteModule } from "./interface";
import Worker from "./web/worker?worker";
import type { SQLiteWorker } from "./web/worker";
import type { CompiledQuery } from "kysely";
import type { SQLiteUpdateEvent } from "./types";

export class SQLiteModuleImpl implements SQLiteModule {
  private worker: Worker;
  private api: Remote<SQLiteWorker>;

  constructor() {
    const worker = new Worker();
    this.worker = worker;
    this.api = wrap<SQLiteWorker>(worker);
  }
  free() {
    this.worker.terminate();
  }
  async create_sqlite(path: string) {
    console.info("创建SQLite实例");
    const db = await this.api.open_db(path);
    return await SQLiteImpl.new(this.api, db);
  }
}

export class SQLiteImpl implements SQLite {
  private api: Remote<SQLiteWorker>;
  private id: number;
  private on_updates: ((event: SQLiteUpdateEvent) => void)[];

  private constructor(
    api: Remote<SQLiteWorker>,
    id: number,
    on_updates: ((event: SQLiteUpdateEvent) => void)[],
  ) {
    this.api = api;
    this.id = id;
    this.on_updates = on_updates;
  }
  static async new(api: Remote<SQLiteWorker>, id: number) {
    const on_updates: ((event: SQLiteUpdateEvent) => void)[] = [];
    await api.on_update(
      id,
      proxy((e) => on_updates.forEach((f) => f(e))),
    );
    return new SQLiteImpl(api, id, on_updates);
  }
  async close() {
    await this.api.close_db(this.id);
  }
  async execute_sql(sql: string) {
    await this.api.execute_sql(this.id, sql);
  }
  async execute(compiled_query: CompiledQuery) {
    await this.api.execute(this.id, compiled_query);
  }
  async query<T>(compiled_query: CompiledQuery) {
    return (await this.api.query(this.id, compiled_query)) as T[];
  }
  on_update(callback: (event: SQLiteUpdateEvent) => void) {
    this.on_updates.push(callback);
    return () => {
      this.on_updates.splice(this.on_updates.indexOf(callback), 1);
    };
  }
}
