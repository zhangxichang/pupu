import { mount } from "svelte"
import {
  start,
  progress,
  done,
  fail,
  finishFirstScreen,
} from "./state.svelte.ts"
import Root from "./Root.svelte"

declare global {
  interface Window {
    WasmInitializer: {
      finishFirstScreen: () => void
    }
  }
}

window.WasmInitializer = { finishFirstScreen }

let app: ReturnType<typeof mount> | undefined

export default () => ({
  onStart() {
    if (!app) {
      let target = document.getElementById("wasm-initializer")

      if (!target) {
        target = document.createElement("div")
        target.id = "wasm-initializer"
        document.body.appendChild(target)
      }

      app = mount(Root, { target })
    }

    start()
  },
  onProgress({ current, total }: { current: number; total: number }) {
    progress(current, total)
  },
  onComplete() { },
  onSuccess() {
    done()
  },
  onFailure(error: unknown) {
    fail(error)
  },
})
