import { expose } from "comlink";
import type { SQLiteAdapter } from "./interface";

export class WebSQLite implements SQLiteAdapter {
  free() {
    self.close();
  }
  async test() {}
}

expose(new WebSQLite());
