import { execa } from "execa";
import path from "node:path";

type CargoMetadata = {
  packages: {
    name: string;
    manifest_path: string;
    targets: {
      crate_types: string[];
    }[];
  }[];
};

const root = path.resolve(import.meta.dir, "..");
const wasmDir = path.join(root, "web/wasm");

const { stdout } = await execa(
  "cargo",
  ["metadata", "--no-deps", "--format-version", "1"],
  { cwd: root },
);
const metadata = JSON.parse(stdout) as CargoMetadata;

const packages = metadata.packages
  .filter((pkg) => {
    const packageDir = path.dirname(pkg.manifest_path);
    const relative = path.relative(wasmDir, packageDir);
    if (relative.startsWith("..") || path.isAbsolute(relative)) return false;
    return pkg.targets.some((target) => target.crate_types.includes("cdylib"));
  })
  .sort((a, b) => a.manifest_path.localeCompare(b.manifest_path));

if (packages.length === 0) {
  console.error(`没有在 ${wasmDir} 找到 cdylib wasm 项目`);
  process.exit(1);
}

for (const pkg of packages) {
  const packageDir = path.dirname(pkg.manifest_path);
  console.log(`构建 wasm：${pkg.name} (${packageDir})`);
  await execa("wasm-pack", ["build", packageDir, "--target", "web"], {
    cwd: root,
    stdio: "inherit",
  });
}
