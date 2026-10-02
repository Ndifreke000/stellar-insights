// Copies the single-file bundle into the Claude plugin, or with --check
// fails if the committed copy is stale. The plugin ships the bundle so it can
// be installed straight from this repository without an npm publish.
import { copyFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const source = join(root, "bundle", "server.mjs");
const target = join(root, "..", "..", "plugins", "payraider", "server", "server.mjs");

if (!existsSync(source)) {
  console.error("bundle/server.mjs not found. Run `npm run build:bundle` first.");
  process.exit(1);
}

if (process.argv.includes("--check")) {
  const fresh = existsSync(target) && readFileSync(source).equals(readFileSync(target));
  if (!fresh) {
    console.error(
      "plugins/payraider/server/server.mjs is out of date. Run `npm run build:plugin` in sdk/mcp-server and commit the result.",
    );
    process.exit(1);
  }
  console.log("plugin bundle is up to date");
} else {
  mkdirSync(dirname(target), { recursive: true });
  copyFileSync(source, target);
  console.log(`wrote ${target}`);
}
