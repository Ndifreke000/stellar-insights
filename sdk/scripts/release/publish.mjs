#!/usr/bin/env node
// Publish every public npm SDK whose current version is not on the registry yet.
//
// Used as the changesets `publish` command instead of `changeset publish`
// because the SDKs are not an npm workspace: each one is installed, built and
// published from its own directory with its own lockfile. The Python SDK is
// published by a separate job in .github/workflows/sdk-release.yml.
//
//   node scripts/release/publish.mjs            # publish
//   node scripts/release/publish.mjs --dry-run  # show what would be published
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// Order matters: dependents build against the local @payraider/sdk.
const PACKAGES = ["typescript", "react", "mcp-server"];

const sdkDir = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const dryRun = process.argv.includes("--dry-run");

const run = (cmd, args, cwd) => execFileSync(cmd, args, { cwd, stdio: "inherit" });

function isPublished(name, version) {
  try {
    execFileSync("npm", ["view", `${name}@${version}`, "version"], { stdio: "pipe" })
      .toString()
      .trim();
    return true;
  } catch {
    return false; // npm view exits non-zero for an unknown version
  }
}

const published = [];
for (const dir of PACKAGES) {
  const cwd = join(sdkDir, dir);
  const pkg = JSON.parse(readFileSync(join(cwd, "package.json"), "utf8"));
  if (pkg.private) continue;

  if (isPublished(pkg.name, pkg.version)) {
    console.log(`skip  ${pkg.name}@${pkg.version} (already on npm)`);
    continue;
  }

  console.log(`publish ${pkg.name}@${pkg.version}${dryRun ? " (dry run)" : ""}`);
  run("npm", ["ci"], cwd);
  run("npm", ["run", "build"], cwd);
  const flags = ["--access", "public"];
  // Provenance needs the GitHub Actions OIDC token; skip it for local runs.
  if (process.env.GITHUB_ACTIONS === "true") flags.push("--provenance");
  if (dryRun) flags.push("--dry-run");
  run("npm", ["publish", ...flags], cwd);
  if (!dryRun) {
    // changesets/action turns lines in this exact format into git tags + releases.
    console.log(`New tag: ${pkg.name}@${pkg.version}`);
  }
  published.push(`${pkg.name}@${pkg.version}`);
}

console.log(published.length ? `Published: ${published.join(", ")}` : "Nothing to publish.");
