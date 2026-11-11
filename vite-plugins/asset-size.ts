import type { Plugin } from "vite";
import fs from "node:fs";
import path from "node:path";

const SIZE_QUERY = "?size";
const SIZE_MODULE_PREFIX = "\0asset-size:";

export default function assetSizePlugin(): Plugin {
  return {
    name: "asset-size",
    enforce: "pre",
    resolveId(source, importer) {
      if (!importer || !source.endsWith(SIZE_QUERY)) return null;
      const file = path.resolve(
        path.dirname(importer),
        source.slice(0, -SIZE_QUERY.length),
      );
      return `${SIZE_MODULE_PREFIX}${file}`;
    },
    load(id) {
      if (!id.startsWith(SIZE_MODULE_PREFIX)) return null;
      const file = id.slice(SIZE_MODULE_PREFIX.length);
      this.addWatchFile(file);
      return `export default ${fs.statSync(file).size};`;
    },
  };
}
