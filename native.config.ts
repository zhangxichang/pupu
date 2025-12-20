/* eslint-disable */
import { defineConfig } from "@solidjs/start/config";
import { config } from "./app.config";

config.server.preset = "static";
config.ssr = false;

export default defineConfig(config);
