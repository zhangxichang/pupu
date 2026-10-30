<script lang="ts">
  import { onMount } from "svelte";
  import type { ResourceState, ResourceStatus } from "./resourceLoading.svelte";
  import {
    resourceLoading,
    startResourceLoading,
  } from "./resourceLoading.svelte";

  const REVEAL_DELAY = 650;
  const MASK_DURATION = 880;
  const CENTER_FADE_DURATION = 360;
  const EXIT_DURATION = MASK_DURATION + CENTER_FADE_DURATION;

  const maskPanels = [
    {
      position: "top-0 left-0 origin-top",
      hidden: "scale-y-0",
      shown: "scale-y-100",
    },
    {
      position: "top-0 right-0 origin-right",
      hidden: "scale-x-0",
      shown: "scale-x-100",
    },
    {
      position: "bottom-0 left-0 origin-left",
      hidden: "scale-x-0",
      shown: "scale-x-100",
    },
    {
      position: "bottom-0 right-0 origin-bottom",
      hidden: "scale-y-0",
      shown: "scale-y-100",
    },
  ];

  const statusText: Record<ResourceStatus, string> = {
    pending: "等待",
    downloading: "下载中",
    initializing: "初始化中",
    waiting: "等待前置资源",
    running: "启动中",
    done: "完成",
    error: "失败",
    blocked: "前置资源失败",
  };

  let visible = $state(true);
  let revealed = $state(false);

  const ready = $derived(
    resourceLoading.resources.length > 0 &&
      resourceLoading.resources.every((resource) => resource.status === "done"),
  );
  const failed = $derived(
    resourceLoading.resources.some((resource) => resource.status === "error"),
  );
  const topResources = $derived(
    resourceLoading.resources.filter(
      (resource) =>
        resource.status === "error" || resource.status === "blocked",
    ),
  );
  const middleResources = $derived(
    resourceLoading.resources.filter(
      (resource) =>
        resource.status !== "error" &&
        resource.status !== "blocked" &&
        resource.status !== "pending" &&
        resource.status !== "waiting" &&
        resource.status !== "done",
    ),
  );
  const doneResources = $derived(
    resourceLoading.resources.filter((resource) => resource.status === "done"),
  );
  const bottomResources = $derived(
    resourceLoading.resources.filter(
      (resource) =>
        resource.status === "pending" || resource.status === "waiting",
    ),
  );

  onMount(() => {
    startResourceLoading();
  });

  $effect(() => {
    if (!ready) return;

    const reveal = window.setTimeout(() => {
      revealed = true;
    }, REVEAL_DELAY);

    return () => window.clearTimeout(reveal);
  });

  $effect(() => {
    if (!revealed) return;

    const finish = window.setTimeout(() => {
      visible = false;
    }, EXIT_DURATION);

    return () => window.clearTimeout(finish);
  });

  function progress(resource: ResourceState): number {
    if (resource.status === "done") return 100;
    if (resource.total <= 0) return 0;
    return Math.min(
      100,
      Math.max(0, (resource.current / resource.total) * 100),
    );
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const units = ["KB", "MB", "GB"];
    let value = bytes / 1024;
    let unit = 0;
    while (value >= 1024 && unit < units.length - 1) {
      value /= 1024;
      unit += 1;
    }
    return `${value.toFixed(1)} ${units[unit]}`;
  }
</script>

{#snippet resourceRow(resource: ResourceState)}
  <div class="flex flex-col gap-1">
    <div
      class="flex items-baseline justify-between gap-3 text-[11px] leading-none"
    >
      <span class="text-[#333]">{resource.label}</span>
      <span class="text-[#999]">
        {statusText[resource.status]}
        {#if resource.status === "downloading"}
          {#if resource.total > 0}
            · {Math.round(progress(resource))}%
          {:else if resource.current > 0}
            · {formatBytes(resource.current)}
          {/if}
        {/if}
      </span>
    </div>
    <div class="h-1 w-full overflow-hidden rounded-full bg-[#e0e0e0]">
      <div
        class="h-full rounded-full bg-[#333]"
        class:animate-pulse={resource.total === 0 && resource.status !== "done"}
        class:opacity-40={resource.total === 0 && resource.status !== "done"}
        style={resource.total === 0 && resource.status !== "done"
          ? "width:100%"
          : `width:${progress(resource)}%`}
      ></div>
    </div>
    {#if resource.error}
      <p class="text-[10px] leading-[1.45] text-[#bb0000] wrap-anywhere">
        {resource.error.message}
      </p>
    {/if}
  </div>
{/snippet}

{#if visible}
  <div
    class="fixed inset-0 grid place-items-center overflow-hidden"
    class:pointer-events-none={revealed}
  >
    {#each maskPanels as mask (mask.position)}
      <div
        class="absolute h-[calc(50%+1px)] w-[calc(50%+1px)] bg-[#f4f4f4] transition-transform duration-880 ease-[cubic-bezier(0.77,0,0.175,1)] {mask.position} {revealed
          ? mask.hidden
          : mask.shown}"
      ></div>
    {/each}

    <div
      class="relative flex w-[78vw] max-w-[320px] flex-col items-center gap-4.5"
      style={`opacity:${revealed ? 0 : 1};transform:scale(${revealed ? 0.94 : 1});transition:opacity 250ms ease ${MASK_DURATION}ms, transform 350ms cubic-bezier(0.22, 1, 0.36, 1) ${MASK_DURATION}ms;`}
    >
      <div
        class="grid size-14 place-items-center rounded-[10px] text-white shadow-[0_4px_16px_rgba(0,0,0,0.16)] transition-[background-color] duration-350 {failed
          ? 'bg-[#bb0000]'
          : 'bg-[#333]'}"
        class:loading-badge-pop={revealed}
      >
        {#if failed}
          <span class="text-[20px] font-semibold leading-none">!</span>
        {:else if revealed}
          <svg class="block size-9.5 loading-mark-in" viewBox="162 160 477 477">
            <g transform="translate(0 800) scale(0.1 -0.1)" fill="currentColor">
              <path
                d="M4725 5702 c-6 -5 -29 -15 -53 -22 -154 -48 -226 -254 -136 -392 l23 -37 -82 -44 c-197 -107 -305 -147 -478 -176 -179 -31 -331 -36 -494 -17 -253 29 -386 36 -505 26 -548 -48 -1034 -363 -1240 -802 -158 -338 -183 -733 -69 -1103 70 -226 154 -359 311 -490 42 -36 93 -89 113 -119 53 -79 144 -160 212 -188 58 -23 61 -23 555 -26 557 -3 551 -4 583 63 35 74 1 191 -81 281 -19 20 -34 38 -34 39 0 1 21 19 47 39 l46 36 334 0 c309 0 334 -1 340 -17 3 -10 23 -56 45 -102 54 -113 160 -222 268 -275 125 -61 156 -66 435 -66 241 0 251 1 290 23 29 16 45 35 59 70 26 61 21 79 -82 312 -44 99 -92 218 -107 265 l-28 86 56 93 c62 103 107 210 130 316 9 39 25 111 36 160 12 54 37 121 61 167 46 90 48 107 17 143 -29 34 -86 38 -118 9 -47 -42 -113 -207 -135 -339 -28 -167 -62 -255 -146 -388 -28 -42 -54 -94 -58 -114 -16 -70 18 -184 136 -459 41 -93 74 -173 74 -177 0 -4 -84 -7 -187 -7 -269 0 -337 19 -451 125 -62 57 -127 164 -153 253 -25 85 -6 82 -479 82 l-417 0 -64 -56 c-76 -69 -177 -118 -269 -134 -55 -9 -74 -17 -90 -37 -31 -39 -26 -86 11 -118 28 -23 37 -25 77 -19 25 4 72 15 105 25 l60 18 31 -27 c38 -33 74 -74 82 -96 6 -15 -33 -16 -417 -16 -372 0 -429 2 -477 17 -64 20 -115 64 -171 148 -22 33 -76 89 -118 125 -92 78 -162 162 -199 239 -69 145 -112 301 -135 488 -22 184 22 451 105 643 144 333 468 594 867 699 208 55 344 60 667 25 252 -27 509 -14 718 36 146 34 287 95 489 209 121 69 176 84 301 82 208 -2 435 -83 810 -291 320 -177 426 -278 470 -450 48 -189 -72 -367 -270 -398 -74 -12 -129 -1 -220 43 -79 39 -110 40 -142 4 -25 -28 -22 -85 7 -115 49 -53 203 -103 314 -104 167 0 360 112 430 250 55 109 69 251 36 375 -63 242 -233 390 -741 646 -179 90 -230 111 -358 150 -72 22 -234 59 -257 59 -2 0 -2 35 -1 78 1 68 -2 83 -28 129 -39 70 -116 124 -186 131 -27 2 -50 8 -50 13 0 11 -13 12 -25 1z m70 -186 c40 -17 55 -42 55 -91 0 -51 -44 -95 -95 -95 -100 0 -131 146 -40 186 38 17 39 17 80 0z"
              />
              <path
                d="M5322 4850 c-114 -26 -130 -175 -25 -225 109 -52 219 86 145 182 -24 32 -80 51 -120 43z"
              />
            </g>
          </svg>
        {:else}
          <span
            class="size-4.5 loading-spin rounded-full border-2 border-[rgba(255,255,255,0.28)] border-t-white"
          ></span>
        {/if}
      </div>

      <div class="flex w-full flex-col gap-2.5">
        {#if topResources.length > 0}
          <div class="flex flex-col gap-2.5">
            {#each topResources as resource (resource.id)}
              {@render resourceRow(resource)}
            {/each}
          </div>
        {/if}

        {#if middleResources.length > 0}
          {#if topResources.length > 0}
            <div class="h-px w-full bg-[#e0e0e0]"></div>
          {/if}
          <div class="flex flex-col gap-2.5">
            {#each middleResources as resource (resource.id)}
              {@render resourceRow(resource)}
            {/each}
          </div>
        {/if}

        {#if bottomResources.length > 0}
          {#if topResources.length > 0 || middleResources.length > 0}
            <div class="h-px w-full bg-[#e0e0e0]"></div>
          {/if}
          <div class="flex flex-col gap-2.5 opacity-50">
            {#each bottomResources as resource (resource.id)}
              {@render resourceRow(resource)}
            {/each}
          </div>
        {/if}

        {#if doneResources.length > 0}
          {#if topResources.length > 0 || middleResources.length > 0 || bottomResources.length > 0}
            <div class="h-px w-full bg-[#e0e0e0]"></div>
          {/if}
          <div class="flex flex-col gap-2.5">
            {#each doneResources as resource (resource.id)}
              {@render resourceRow(resource)}
            {/each}
          </div>
        {/if}

        {#if failed}
          <p class="pt-1 text-center text-[10px] leading-[1.45] text-[#888]">
            加载失败，请刷新页面
          </p>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .loading-spin {
    animation: loading-spin 0.8s linear infinite;
  }

  .loading-badge-pop {
    animation: loading-badge-pop 0.65s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .loading-mark-in {
    animation: loading-mark-in 0.6s cubic-bezier(0.22, 1, 0.36, 1) both;
  }

  @keyframes loading-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes loading-badge-pop {
    0% {
      transform: scale(1);
    }
    45% {
      transform: scale(1.08);
    }
    100% {
      transform: scale(1);
    }
  }

  @keyframes loading-mark-in {
    from {
      opacity: 0;
      transform: scale(0.6) rotate(-24deg);
    }
    to {
      opacity: 1;
      transform: scale(1) rotate(0deg);
    }
  }
</style>
