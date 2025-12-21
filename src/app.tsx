import "~/app.css";
import { Router } from "@solidjs/router";
import { FileRoutes } from "@solidjs/start/router";
import { children, ErrorBoundary, Suspense } from "solid-js";
import Loading from "./components/widgets/loading";
import MenuBar from "./components/ui/menu_bar";
import Error from "./components/widgets/error";

export default function App() {
  return (
    <Router
      root={(props) => {
        const resolved = children(() => props.children);
        return (
          <div class="absolute w-dvw h-dvh flex flex-col bg-base-200">
            <MenuBar />
            <ErrorBoundary
              fallback={(error) => <Error error={error as Error} />}
            >
              <Suspense fallback={<Loading />}>{resolved()}</Suspense>
            </ErrorBoundary>
          </div>
        );
      }}
    >
      <FileRoutes />
    </Router>
  );
}
