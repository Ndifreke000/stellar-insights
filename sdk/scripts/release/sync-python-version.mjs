#!/usr/bin/env node
// Copy the version changesets wrote to python/package.json into pyproject.toml.
// Runs as part of `npm run version-packages`; safe to re-run (no-op when equal).
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const sdkDir = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const { version } = JSON.parse(readFileSync(join(sdkDir, "python", "package.json"), "utf8"));
const pyprojectPath = join(sdkDir, "python", "pyproject.toml");
const pyproject = readFileSync(pyprojectPath, "utf8");

// Only the `version` key inside the [project] table.
const projectTable = /(\[project\][^[]*?^version\s*=\s*)"([^"]*)"/ms;
const match = pyproject.match(projectTable);
if (!match) {
  console.error(`sync-python-version: no [project] version in ${pyprojectPath}`);
  process.exit(1);
}
if (match[2] === version) {
  console.log(`sync-python-version: pyproject.toml already at ${version}`);
} else {
  writeFileSync(pyprojectPath, pyproject.replace(projectTable, `$1"${version}"`));
  console.log(`sync-python-version: pyproject.toml ${match[2]} -> ${version}`);
}
