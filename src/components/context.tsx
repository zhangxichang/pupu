import type { Remote } from "comlink";
import { createContext, useContext } from "solid-js";
import type { SQLiteAdapter } from "~/lib/sqlite/interface";

export interface MainState {
  db: SQLiteAdapter | Remote<SQLiteAdapter>;
}
export const MainContext = createContext<MainState>();
export function use_main() {
  const state = useContext(MainContext);
  if (!state) throw new Error("上下文不存在");
  return state;
}
