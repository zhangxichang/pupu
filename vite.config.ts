import { defineConfig } from "vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import solid from "vite-plugin-solid";
import tailwindcss from "@tailwindcss/vite";
import { visualizer } from "rollup-plugin-visualizer";

export default defineConfig({
  clearScreen: false,
  build: {
    target: "esnext",
  },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  plugins: [
    tanstackRouter({ target: "solid", autoCodeSplitting: true }),
    solid(),
    tailwindcss(),
    visualizer({
      filename: "pkg_view.html",
    }),
  ],
});
