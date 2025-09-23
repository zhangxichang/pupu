import "./__root.css";
import { createRootRoute, HeadContent, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect } from "react";

export const Route = createRootRoute({
    component: Component,
    head: () => ({
        meta: [{ title: "星链" }]
    }),
});
function Component() {
    useEffect(() => {
        if (isTauri()) {
            getCurrentWindow().setTitle(document.title);
            const mutation_observer = new MutationObserver((mutations) => {
                for (const mutation of mutations) {
                    if (mutation.type === "childList") {
                        getCurrentWindow().setTitle(document.title).catch(console.error);
                    }
                }
            });
            mutation_observer.observe(document.querySelector("title")!, { childList: true });
            return () => mutation_observer.disconnect();
        }
    }, []);
    return <>
        <HeadContent />
        <Outlet />
        <TanStackRouterDevtools />
    </>;
}
