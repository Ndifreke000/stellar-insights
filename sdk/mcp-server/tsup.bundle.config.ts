import { defineConfig } from "tsup";

/**
 * Single-file build with every dependency inlined. This is what the Docker
 * image and the Claude plugin run, so neither needs `npm install`.
 */
export default defineConfig({
  entry: { server: "src/index.ts" },
  format: ["esm"],
  platform: "node",
  target: "node20",
  outDir: "bundle",
  outExtension: () => ({ js: ".mjs" }),
  noExternal: [/.*/],
  splitting: false,
  dts: false,
  clean: true,
  minify: false,
  // Some inlined CommonJS dependencies call require() at runtime.
  banner: {
    js: 'import { createRequire as __createRequire } from "node:module"; const require = __createRequire(import.meta.url);',
  },
});
