import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider, createHashHistory, createRouter } from "@tanstack/react-router";
import { routeTree } from "./routeTree.gen";

const router = createRouter({ routeTree, history: createHashHistory() });
declare module "@tanstack/react-router" {
    interface Register {
        router: typeof router
    }
}

export function render() {
    createRoot(document.body).render(<StrictMode>
        <RouterProvider router={router} />
    </StrictMode>);
}
