#!/usr/bin/env node
// Build the local @payraider/sdk (sdk/typescript) if it has not been built yet.
//
// sdk/react and sdk/mcp-server depend on "@payraider/sdk": "file:../typescript",
// which npm installs as a symlink to the source folder. The package resolves
// through dist/ (types, main, module), so building, type-checking or testing a
// dependent before sdk/typescript is built fails ("Cannot find module
// '@payraider/sdk'" / tsup "DTS Build error"). Dependents run this from their
// pre* scripts so the build order no longer has to be remembered.
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const sdkDir = join(dirname(fileURLToPath(import.meta.url)), "..", "typescript");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const run = (args) => execFileSync(npm, args, { cwd: sdkDir, stdio: "inherit" });

const built = ["index.js", "index.mjs", "index.d.ts"].every((f) => existsSync(join(sdkDir, "dist", f)));
if (!built) {
  if (!existsSync(join(sdkDir, "node_modules"))) {
    console.log("[ensure-local-sdk] installing sdk/typescript dependencies");
    run(["ci", "--no-audit", "--no-fund"]);
  }
  console.log("[ensure-local-sdk] building sdk/typescript");
  run(["run", "build"]);
}
