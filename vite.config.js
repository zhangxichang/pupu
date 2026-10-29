import { defineConfig } from "vite"
import { svelte, vitePreprocess } from "@sveltejs/vite-plugin-svelte"

export default defineConfig({
  plugins: [svelte({ preprocess: vitePreprocess() })],
  build: {
    outDir: "build-dist",
    lib: {
      entry: "wasm-initializer/lib.ts",
      formats: ["es"],
      fileName: "wasm-initializer",
      cssFileName: "wasm-initializer",
    },
  },
})
