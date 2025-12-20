import "~/app.css";
import { Router } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import { ErrorBoundary, Suspense } from "solid-js";
import MenuBar from "./components/ui/menu_bar";
import Error from "./components/widgets/error";
import Loading from "./components/widgets/loading";

export default function App() {
  return (
    <Router
      root={(props) => (
        <div class="absolute w-dvw h-dvh flex flex-col bg-base-200">
          <MenuBar />
          <ErrorBoundary fallback={(error) => <Error error={error as Error} />}>
            <Suspense fallback={<Loading />}>{props.children}</Suspense>
          </ErrorBoundary>
        </div>
      )}
    >
      <FileRoutes />
    </Router>
  );
}
