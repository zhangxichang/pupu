import { createAsync, type RouteSectionProps } from "@solidjs/router";
import { onCleanup, Show } from "solid-js";
import { MainContext, type MainState } from "~/components/context";
import { create_sqlite } from "~/lib/sqlite";

export default function Main(props: RouteSectionProps) {
  const state = createAsync<MainState>(create_state);
  return (
    <Show keyed when={state()}>
      {(v) => {
        onCleanup(async () => await cleanup_state(v));
        return (
          <MainContext.Provider value={v}>
            {props.children}
          </MainContext.Provider>
        );
      }}
    </Show>
  );
}
async function create_state() {
  const db = create_sqlite();
  await db.init();
  await db.open("data.db");
  return {
    db,
  };
}
async function cleanup_state(state: MainState) {
  await state.db.close();
  await state.db.free();
}
