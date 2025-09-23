import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

export default defineConfig({
    clearScreen: false,
    plugins: [tailwindcss(), tanstackRouter({ target: "react", autoCodeSplitting: true }), react()],
    resolve: {
        alias: {
            "@": path.resolve(__dirname, "./src")
        }
    }
});
