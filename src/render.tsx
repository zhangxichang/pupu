import {
  RouterProvider,
  createMemoryHistory,
  createRouter,
} from "@tanstack/solid-router";
import { routeTree } from "./routeTree.gen";
import { render } from "solid-js/web";

const Router = createRouter({
  routeTree,
  history: createMemoryHistory({
    initialEntries: ["/window"],
  }),
});
declare module "@tanstack/solid-router" {
  interface Register {
    router: typeof Router;
  }
}

render(() => <RouterProvider router={Router} />, document.body);
