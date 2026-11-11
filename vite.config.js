import { defineConfig } from "vite";
import { svelte, vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import assetSizePlugin from "./vite-plugins/asset-size.ts";

export default defineConfig({
  plugins: [
    assetSizePlugin(),
    svelte({ preprocess: vitePreprocess() }),
    tailwindcss(),
  ],
  worker: { format: "es" },
});
