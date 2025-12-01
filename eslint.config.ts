import { defineConfig } from "eslint/config";
import eslint from "@eslint/js";
import tseslint from "typescript-eslint";

export default defineConfig(
  { ignores: ["src/shadcn/**"] },
  eslint.configs.recommended,
  tseslint.configs.recommendedTypeChecked,
  { languageOptions: { parserOptions: { projectService: true } } },
  {
    rules: {
      "@typescript-eslint/strict-boolean-expressions": "error",
    },
  },
);
