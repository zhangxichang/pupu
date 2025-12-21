import { wrap } from "comlink";
import type { SQLiteAdapter } from "./sqlite/interface";
import Worker from "./sqlite/web?worker";

type Native = { kind: "Native" } & typeof import("./sqlite/native");
type Web = { kind: "Web" };

let api: Native | Web;
if (import.meta.env.TAURI_ENV_PLATFORM !== undefined) {
  api = { kind: "Native", ...(await import("./sqlite/native")) };
} else {
  api = { kind: "Web" };
}

export function create_sqlite(): SQLiteAdapter {
  if (api.kind === "Native") {
    return new api.NativeSQLite();
  } else if (api.kind === "Web") {
    return wrap<SQLiteAdapter>(new Worker());
  } else {
    throw new Error("API缺失");
  }
}
