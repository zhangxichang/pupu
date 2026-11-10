import { execa } from "execa";
import { globby } from "globby";

const ignoreFiles = await globby("**/.gitignore", {
  dot: true,
  gitignore: true,
  absolute: true,
});
const ignoreArgs = ignoreFiles.flatMap((file) => ["--ignore-path", file]);
const result = await execa("prettier", ["--write", ...ignoreArgs, "."], {
  stdio: "inherit",
  preferLocal: true,
  reject: false,
});
process.exitCode = result.exitCode ?? 1;
