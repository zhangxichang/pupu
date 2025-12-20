import { defineConfig } from "@solidjs/start/config";
import tailwindcss from "@tailwindcss/vite";
import { visualizer } from "rollup-plugin-visualizer";

export default defineConfig({
  server: {
    preset: "cloudflare_module",
    rollupConfig: {
      external: ["__STATIC_CONTENT_MANIFEST", "node:async_hooks"],
    },
    esbuild: { options: { target: "esnext" } },
  },
  vite: {
    build: {
      target: "esnext",
    },
    envPrefix: ["VITE_", "TAURI_ENV_"],
    plugins: [tailwindcss(), visualizer()],
  },
});
