import type { ResourceInfo } from "../ResourceLoading/types";
import storageWorkerWasmUrl from "../../wasm/storage-worker/pkg/storage_worker_bg.wasm?url";
import storageWorkerWasmSize from "../../wasm/storage-worker/pkg/storage_worker_bg.wasm?size";
import storageWorkerUrl from "./storage/worker.ts?worker&url";

export const storageWorkerEntryUrl = new URL(storageWorkerUrl, location.origin)
  .href;

export function storageResource(): ResourceInfo {
  return {
    kind: "basic",
    id: "storage",
    label: "存储",
    url: storageWorkerWasmUrl,
    size: storageWorkerWasmSize,
  };
}
