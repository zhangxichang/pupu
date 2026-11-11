import type { ResourceInfo } from "../ResourceLoading/types";
import initMain, { main } from "../../wasm/main/pkg";
import mainWasmUrl from "../../wasm/main/pkg/main_bg.wasm?url";
import mainWasmSize from "../../wasm/main/pkg/main_bg.wasm?size";
import { takeFontData } from "./font";
import { storageWorkerEntryUrl } from "./storage";

const MAIN_START_TIMEOUT_MS = 120_000;

export function mainResource(): ResourceInfo {
  return {
    kind: "wasm",
    id: "main",
    label: "主程序",
    url: mainWasmUrl,
    size: mainWasmSize,
    init: initMain,
    onStart: startMain,
    dependsOn: [
      { id: "storage", phase: "download" },
      { id: "font", phase: "download" },
    ],
  };
}

async function startMain(): Promise<unknown> {
  const mainPromise = main(takeFontData(), storageWorkerEntryUrl);
  let timer: number | undefined;

  try {
    return await Promise.race([
      mainPromise,
      new Promise<never>((_, reject) => {
        timer = window.setTimeout(
          () => reject(new Error("main 启动超时")),
          MAIN_START_TIMEOUT_MS,
        );
      }),
    ]);
  } finally {
    if (timer !== undefined) {
      window.clearTimeout(timer);
    }
  }
}
