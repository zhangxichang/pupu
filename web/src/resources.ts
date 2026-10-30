// TODO AI实现待审查
import mainFontUrl from "../../assets/fonts/MiSans VF.ttf?url";
import mainWasmUrl from "../wasm/main/pkg/main_bg.wasm?url";
import initMain, { main } from "../wasm/main/pkg";
import storageWorkerWasmUrl from "../wasm/storage-worker/pkg/storage_worker_bg.wasm?url";
import storageWorkerUrl from "./storage.worker.ts?worker&url";

export type WasmInit = (input: {
  module_or_path: Response;
}) => Promise<unknown>;

export type ResourcePhase = "download" | "execute";

export type ResourceDependency = {
  id: string;
  phase: ResourcePhase;
};

export type AppResource = {
  id: string;
  label: string;
  url: string;
  init?: WasmInit;
  start?: () => Promise<unknown>;
  onDownload?: (bytes: ArrayBuffer) => void;
  dependsOn?: readonly ResourceDependency[];
};

const storageWorkerEntryUrl = new URL(storageWorkerUrl, location.origin).href;

const MAIN_START_TIMEOUT_MS = 120_000;

let fontData: Uint8Array | undefined;

function takeFontData(): Uint8Array {
  if (!fontData) {
    throw new Error("字体数据缺失");
  }
  const data = fontData;
  fontData = undefined;
  return data;
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

export const resourceDefinitions: readonly AppResource[] = [
  {
    id: "storage",
    label: "存储",
    url: storageWorkerWasmUrl,
  },
  {
    id: "font",
    label: "字体",
    url: mainFontUrl,
    onDownload: (bytes) => {
      fontData = new Uint8Array(bytes);
    },
  },
  {
    id: "main",
    label: "主程序",
    url: mainWasmUrl,
    init: initMain,
    start: startMain,
    dependsOn: [
      { id: "storage", phase: "download" },
      { id: "font", phase: "download" },
    ],
  },
];
