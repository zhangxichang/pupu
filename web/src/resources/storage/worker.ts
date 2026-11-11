import storageWasmUrl from "../../../wasm/storage-worker/pkg/storage_worker_bg.wasm?url";
import init, { register } from "../../../wasm/storage-worker/pkg";

await init({
  module_or_path: new URL(storageWasmUrl, self.location.origin).href,
});
register();
