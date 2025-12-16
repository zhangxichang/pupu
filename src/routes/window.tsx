import { createFileRoute, Outlet } from "@tanstack/solid-router";
import { Maximize, Minimize, Minimize2, X } from "lucide-solid";
import { Octokit } from "octokit";
import { createSignal, For, onCleanup, onMount } from "solid-js";
import { themeChange } from "theme-change";
import { open_url } from "../lib/opener";

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
    let about_dialog: HTMLDialogElement | undefined;
    const [is_maximized, set_is_maximized] = createSignal<boolean>();
    const [version, set_version] = createSignal<string>();
    const [contributors, set_contributors] = createSignal<
      {
        avatar_url?: string;
      }[]
    >();
    onMount(() => {
      themeChange();
      //获取版本
      void (async () => {
        const version = await fetch("/version");
        const content_type = version.headers.get("Content-Type");
        if (content_type !== "text/html") {
          set_version(await version.text());
        }
      })();
      //获取贡献者信息
      void (async () => {
        set_contributors(
          (
            await new Octokit().rest.repos.listContributors({
              owner: "ZhangXiChang",
              repo: "starlink",
            })
          ).data.map((v) => ({
            avatar_url: v.avatar_url,
          })),
        );
      })();
      const promise = Promise.all([
        (async () => {
          if (tauri_window) {
            //设置窗口标题
            await tauri_window.getCurrentWindow().setTitle(document.title);
            //同步标题变化
            const title_observer = new MutationObserver(
              () =>
                void tauri_window.getCurrentWindow().setTitle(document.title),
            );
            title_observer.observe(document.querySelector("title")!, {
              childList: true,
              characterData: true,
            });
            //设置窗口缩放状态
            set_is_maximized(
              await tauri_window.getCurrentWindow().isMaximized(),
            );
            //监控窗口缩放
            const un_on_resized = await tauri_window
              .getCurrentWindow()
              .onResized(
                () =>
                  void (async () => {
                    set_is_maximized(
                      await tauri_window.getCurrentWindow().isMaximized(),
                    );
                  })(),
              );
            return { title_observer, un_on_resized };
          }
        })(),
      ]);
      onCleanup(async () => {
        const cleanup = (await promise)[0];
        cleanup?.title_observer.disconnect();
        cleanup?.un_on_resized();
      });
    });
    return (
      <div class="absolute w-dvw h-dvh flex flex-col bg-base-200">
        <div class="flex items-start">
          {/* 菜单栏 */}
          <ul class="menu menu-horizontal">
            <li>
              <button
                class="btn btn-sm"
                onClick={() => about_dialog?.showModal()}
              >
                关于
              </button>
            </li>
          </ul>
          <dialog ref={about_dialog} class="modal">
            <div class="modal-box flex flex-col">
              <span class="text-base-content font-bold text-lg">关于</span>
              <span class="label text-sm">两地俱秋夕，相望共星河。</span>
              <div class="flex flex-col mt-3 gap-2">
                <div class="flex flex-col">
                  <span
                    class="link link-hover font-bold"
                    onClick={() =>
                      void open_url(
                        "https://github.com/ZhangXiChang/starlink/graphs/contributors",
                      )
                    }
                  >
                    贡献者名单
                  </span>
                  <div class="avatar-group justify-center -space-x-6">
                    <For each={contributors()}>
                      {(v) => (
                        <div class="avatar">
                          <img class="size-10" src={v.avatar_url} />
                        </div>
                      )}
                    </For>
                  </div>
                </div>
                <span
                  class="link link-hover text-sm font-bold text-info"
                  onClick={() =>
                    void open_url("https://github.com/ZhangXiChang/starlink")
                  }
                >
                  源码仓库
                </span>
                <span class="label text-sm">
                  版本号：{version() ?? "开发版本"}
                </span>
              </div>
              <div class="modal-action">
                <div class="label text-sm">
                  按<kbd class="kbd kbd-sm">ESC</kbd>关闭
                </div>
              </div>
            </div>
          </dialog>
          {/* 窗口控制栏 */}
          {tauri_window && (
            <div data-tauri-drag-region class="flex-1 flex justify-end">
              <button
                class="btn btn-square btn-ghost btn-sm rounded-none"
                onClick={() => void tauri_window.getCurrentWindow().minimize()}
              >
                <Minimize2 size={"16px"} />
              </button>
              <button
                class="btn btn-square btn-ghost btn-sm rounded-none"
                onClick={() =>
                  void tauri_window.getCurrentWindow().toggleMaximize()
                }
              >
                {is_maximized() === true ? (
                  <Minimize size={"16px"} />
                ) : (
                  <Maximize size={"16px"} />
                )}
              </button>
              <button
                class="btn btn-square btn-ghost btn-sm rounded-none btn-error text-base-content"
                onClick={() => void tauri_window.getCurrentWindow().close()}
              >
                <X size={"16px"} />
              </button>
            </div>
          )}
        </div>
        <Outlet />
      </div>
    );
  },
});
