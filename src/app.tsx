import "~/app.css";
import { Router, useIsRouting, type RouteSectionProps } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import MenuBar from "./components/ui/menu_bar";
import { ErrorBoundary, Show, Suspense } from "solid-js";
import Error from "./components/widgets/error";
import Loading from "./components/widgets/loading";

if (import.meta.env.TAURI_ENV_PLATFORM !== undefined) {
  const { createTauRPCProxy } = await import("~/generated/ipc_bindings");
  const log_info = console.info;
  console.info = (message: string) => {
    log_info(message);
    void createTauRPCProxy().log.info(message);
  };
  const log_warn = console.warn;
  console.warn = (message: string) => {
    log_warn(message);
    void createTauRPCProxy().log.warn(message);
  };
  const log_error = console.error;
  console.error = (error: Error) => {
    log_error(error);
    void createTauRPCProxy().log.error(error.message);
  };
  window.onunhandledrejection = (e) => {
    e.preventDefault();
    console.error(e.reason);
  };
}

export function Shell(props: RouteSectionProps) {
  const is_routing = useIsRouting();
  return (
    <div class="absolute w-dvw h-dvh flex flex-col bg-base-200">
      <MenuBar />
      <div class="flex-1 flex relative min-h-0">
        <Show when={is_routing()}>
          <progress class="progress progress-primary absolute rounded-none h-0.5" />
        </Show>
        <ErrorBoundary fallback={(error) => <Error error={error as Error} />}>
          <Suspense fallback={<Loading />}>{props.children}</Suspense>
        </ErrorBoundary>
      </div>
    </div>
  );
}
export default function App() {
  return (
    <Router root={Shell}>
      <FileRoutes />
    </Router>
  );
}
