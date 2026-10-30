import { defineConfig } from "eslint/config";
import js from "@eslint/js";
import ts from "typescript-eslint";
import globals from "globals";
import gitignore from "eslint-config-flat-gitignore";
import prettier from "eslint-config-prettier";
import svelte from "eslint-plugin-svelte";

export default defineConfig(
  gitignore({ recursive: true }),
  {
    files: ["**/*.ts", "**/*.svelte"],
    extends: [
      js.configs.recommended,
      ts.configs.recommended,
      svelte.configs.recommended,
      prettier,
      svelte.configs.prettier,
    ],
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.worker,
        ...globals.svelte,
      },
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts"],
    rules: {
      "no-undef": "off",
    },
  },
  {
    files: ["**/*.ts", "**/*.svelte"],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
        projectService: true,
        extraFileExtensions: [".svelte"],
      },
    },
    rules: {
      "@typescript-eslint/no-floating-promises": [
        "error",
        { checkThenables: true, ignoreIIFE: true },
      ],
    },
  },
);
