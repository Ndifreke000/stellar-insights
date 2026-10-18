// Copies the single-file bundle into plugins/payraider/server so the plugin
// can be run locally (`claude --plugin-dir plugins/payraider`). The copy is
// gitignored: CI builds it and publishes the plugin to the plugin-dist branch,
// which is what the marketplace installs from.
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const source = join(root, "bundle", "server.mjs");
const target = join(root, "..", "..", "plugins", "payraider", "server", "server.mjs");

if (!existsSync(source)) {
  console.error("bundle/server.mjs not found. Run `npm run build:bundle` first.");
  process.exit(1);
}

mkdirSync(dirname(target), { recursive: true });
copyFileSync(source, target);
console.log(`wrote ${target}`);
