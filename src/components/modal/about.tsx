import { ErrorBoundary, For, Suspense } from "solid-js";
import { open_url } from "~/lib/opener";
import { createAsync } from "@solidjs/router";
import { get_contributors, get_version } from "~/query";
import Image from "../widgets/image";
import Loading from "../widgets/loading";
import Error from "../widgets/error";

export default function AboutModal() {
  const version = createAsync(() => get_version());
  const contributors = createAsync(() => get_contributors());
  return (
    <div class="modal-box flex flex-col relative">
      <span class="text-base-content font-bold text-lg">关于</span>
      <span class="label text-sm">两地俱秋夕，相望共星河。</span>
      <div class="flex flex-col mt-3 gap-2">
        <div class="flex flex-col items-start">
          <span
            class="link link-hover font-bold text-accent"
            onClick={() =>
              open_url(
                "https://github.com/ZhangXiChang/starlink/graphs/contributors",
              )
            }
          >
            贡献者们
          </span>
          <div class="avatar-group w-full justify-center -space-x-6">
            <ErrorBoundary
              fallback={(error) => <Error error={error as Error} />}
            >
              <Suspense fallback={<Loading />}>
                <For each={contributors()}>
                  {(v) => (
                    <div class="avatar">
                      <Image class="size-10" image={v.avatar_url} />
                    </div>
                  )}
                </For>
              </Suspense>
            </ErrorBoundary>
          </div>
        </div>
        <div class="flex flex-col items-start">
          <span
            class="link link-hover text-sm font-bold text-info"
            onClick={() => open_url("https://github.com/ZhangXiChang/starlink")}
          >
            源码仓库
          </span>
          <span class="label text-sm">版本号：{version() ?? "开发版本"}</span>
        </div>
      </div>
      <span class="absolute right-4 bottom-3 label text-sm">
        按<kbd class="kbd kbd-sm">ESC</kbd>关闭
      </span>
    </div>
  );
}
