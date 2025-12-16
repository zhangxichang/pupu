import { createFileRoute, Outlet } from "@tanstack/solid-router";
import { onCleanup, onMount } from "solid-js";
import { themeChange } from "theme-change";
import { MenuBar } from "../components/ui/menu_bar";

let tauri_window: typeof import("@tauri-apps/api/window") | undefined;
if (import.meta.env.TAURI_ENV_PLATFORM !== undefined) {
  tauri_window = await import("@tauri-apps/api/window");
}

export const Route = createFileRoute("/window")({
  pendingComponent: () => {
    return (
      <div class="absolute w-dvw h-dvh flex items-center justify-center">
        <span class="loading" />
      </div>
    );
  },
  component: () => {
    onMount(() => {
      themeChange();
      if (tauri_window) {
        //设置窗口标题
        void tauri_window.getCurrentWindow().setTitle(document.title);
        //同步标题变化
        const title_observer = new MutationObserver(() =>
          tauri_window.getCurrentWindow().setTitle(document.title),
        );
        title_observer.observe(document.querySelector("title")!, {
          childList: true,
          characterData: true,
        });
        onCleanup(() => title_observer.disconnect());
      }
    });
    return (
      <div class="absolute w-dvw h-dvh flex flex-col bg-base-200">
        <MenuBar />
        <Outlet />
      </div>
    );
  },
});
