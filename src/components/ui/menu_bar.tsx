import { open_url } from "../../lib/opener";
import { createSignal, For, onMount } from "solid-js";
import { Octokit } from "octokit";
import { WindowControlBar } from "./window_control_bar";

let tauri_window: typeof import("@tauri-apps/api/window") | undefined;
if (import.meta.env.TAURI_ENV_PLATFORM !== undefined) {
  tauri_window = await import("@tauri-apps/api/window");
}

export function MenuBar() {
  let about_dialog: HTMLDialogElement | undefined;
  const [version, set_version] = createSignal<string>();
  const [contributors, set_contributors] = createSignal<
    {
      avatar_url?: string;
    }[]
  >();
  onMount(async () => {
    //获取版本
    const version = await fetch("/version");
    const content_type = version.headers.get("Content-Type");
    if (content_type !== "text/html") {
      set_version(await version.text());
    }
    //获取贡献者信息
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
  });
  return (
    <div class="flex items-start">
      {/* 菜单栏 */}
      <ul class="menu menu-horizontal">
        <li>
          <button class="btn btn-sm" onClick={() => about_dialog?.showModal()}>
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
                  open_url(
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
                open_url("https://github.com/ZhangXiChang/starlink")
              }
            >
              源码仓库
            </span>
            <span class="label text-sm">版本号：{version() ?? "开发版本"}</span>
          </div>
          <div class="modal-action">
            <div class="label text-sm">
              按<kbd class="kbd kbd-sm">ESC</kbd>关闭
            </div>
          </div>
        </div>
      </dialog>
      {tauri_window && <WindowControlBar tauri_window={tauri_window} />}
    </div>
  );
}
