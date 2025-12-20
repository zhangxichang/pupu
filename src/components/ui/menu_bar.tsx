import {
  createSignal,
  ErrorBoundary,
  lazy,
  onMount,
  Show,
  Suspense,
} from "solid-js";
import WindowControlBar from "./window_control_bar";
import { get_window } from "~/lib/window";
import type { Window } from "@tauri-apps/api/window";
import Error from "../widgets/error";
import Loading from "../widgets/loading";

const LazyAboutModal = lazy(() => import("~/components/modal/about"));

export default function MenuBar() {
  let about_dialog: HTMLDialogElement | undefined;
  const [lazy_about_modal_load, set_lazy_about_modal_load] =
    createSignal(false);
  const [window, set_window] = createSignal<Window>();
  onMount(() => set_window(get_window()));
  return (
    <div class="flex items-start">
      <ul class="menu menu-horizontal">
        <li>
          <button
            class="btn btn-sm"
            onClick={() => {
              about_dialog?.showModal();
              set_lazy_about_modal_load(true);
            }}
          >
            关于
          </button>
        </li>
      </ul>
      <dialog ref={about_dialog} class="modal">
        <Show when={lazy_about_modal_load()}>
          <ErrorBoundary
            fallback={(error) => (
              <div class="modal-box flex">
                <Error error={error as Error} />
              </div>
            )}
          >
            <Suspense
              fallback={
                <div class="modal-box flex">
                  <Loading />
                </div>
              }
            >
              <LazyAboutModal />
            </Suspense>
          </ErrorBoundary>
        </Show>
      </dialog>
      <Show when={window()}>{(v) => <WindowControlBar window={v()} />}</Show>
    </div>
  );
}
