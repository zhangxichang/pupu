import { defineConfig } from "vite";
import path from "path";
import react from "@vitejs/plugin-react";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import tailwindcss from "@tailwindcss/vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
    clearScreen: false,
    plugins: [wasm(), tailwindcss(), tanstackRouter({ target: "react", autoCodeSplitting: true }), react()],
    resolve: {
        alias: {
            "@": path.resolve(__dirname, "./src")
        }
    }
});
