import { defineConfig } from "vite";
import { svelte, vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [svelte({ preprocess: vitePreprocess() }), tailwindcss()],
  worker: { format: "es" },
});
