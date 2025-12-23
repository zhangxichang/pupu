import type { Remote } from "comlink";
import type { SQLiteAdapter } from "./sqlite/interface";

type Native = { kind: "Native" };
type Web = {
  kind: "Web";
  worker: typeof import("*?worker");
} & typeof import("comlink");

let api: Native | Web;
if (import.meta.env.TAURI_ENV_PLATFORM !== undefined) {
  api = { kind: "Native" };
} else {
  api = {
    kind: "Web",
    worker: await import("./sqlite/web?worker"),
    ...(await import("comlink")),
  };
}

export function create_sqlite(): SQLiteAdapter | Remote<SQLiteAdapter> {
  if (api.kind === "Native") {
    throw new Error("未实现");
  } else if (api.kind === "Web") {
    return api.wrap<SQLiteAdapter>(new api.worker.default());
  } else {
    throw new Error("API缺失");
  }
}
