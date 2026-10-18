# TypeScript SDK: Move Build/Publishing Helpers Out of `src`

**GitHub Issue**: [#2585](https://github.com/Ndifreke000/stellar-insights/issues/2585)  
**Estimated size**: S — roughly half a day to a day for someone familiar with the stack.

---

## Overview

Two files currently live inside `sdk/typescript/src/` but are not part of the published SDK runtime:

| File | Current location | Problem |
|---|---|---|
| `npm_publishing_setup.ts` | `src/npm_publishing_setup.ts` | A build/publish helper that ships in the package tarball, bloating installs |
| `sdk_unit_tests.ts` | `src/sdk_unit_tests.ts` | A test fixture that ships in the package tarball, confusing consumers |

This change moves each file to the correct directory and updates `package.json` so the published tarball contains only runtime code.

---

## Current State

```
sdk/typescript/
├── src/                          ← everything here is published by default
│   ├── npm_publishing_setup.ts   ← should NOT be published
│   ├── sdk_unit_tests.ts         ← should NOT be published
│   ├── index.ts
│   ├── api-client.ts
│   └── ...
├── scripts/
├── tests/
├── package.json                  ← "files": ["dist"] (correct, but src is compiled into dist)
└── tsconfig.json
```

The `package.json` already sets `"files": ["dist"]`, which means only the compiled output is published. However, `npm_publishing_setup.ts` and `sdk_unit_tests.ts` are compiled along with the runtime sources (via `tsup src/index.ts`) and may transitively end up in `dist` if they are imported anywhere, or they pollute the TypeScript compilation and confuse tooling.

The canonical fix is to remove these files from `src` so they are never compiled into `dist` in the first place.

---

## Target Layout

```
sdk/typescript/
├── src/                          ← runtime code only
│   ├── index.ts
│   ├── api-client.ts
│   └── ...
├── scripts/
│   └── npm_publishing_setup.ts   ← moved here
├── tests/
│   └── sdk_unit_tests.ts         ← moved here (alongside existing test files)
├── package.json
└── tsconfig.json
```

---

## Implementation Steps

### 1. Move files

```bash
# from sdk/typescript/
mv src/npm_publishing_setup.ts scripts/npm_publishing_setup.ts
mv src/sdk_unit_tests.ts       tests/sdk_unit_tests.ts
```

### 2. Update `tsconfig.json`

Verify that `scripts/` and `tests/` are excluded from the main compilation:

```jsonc
// tsconfig.json
{
  "compilerOptions": { ... },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "scripts", "tests"]
}
```

If `tests/` is already excluded (common for vitest setups that use a separate `tsconfig.test.json`), no change is needed for that directory. Add `scripts/` to the exclude list if it is not already there.

### 3. Verify `package.json` `files` field

The existing `"files": ["dist"]` is correct. After the move, confirm no other entry inadvertently includes `src/` or `scripts/`:

```json
{
  "files": ["dist"]
}
```

Run a dry-run pack to confirm the tarball contents:

```bash
npm pack --dry-run
```

The output should list only files under `dist/`, plus `package.json`, `README.md`, and `LICENSE`. It must **not** include `npm_publishing_setup.*` or `sdk_unit_tests.*`.

### 4. Update internal imports

If `npm_publishing_setup.ts` or `sdk_unit_tests.ts` import from `../src/...`, update those paths to reflect the new location:

```ts
// scripts/npm_publishing_setup.ts — before
import { something } from '../src/index';

// after (no path change needed if relative paths still resolve correctly,
// but verify after the move)
import { something } from '../src/index';
```

Because both files are moving *out of* `src/`, any import of the form `./api-client` inside them becomes `../src/api-client`. Check and update accordingly.

### 5. Update CI / scripts that reference the old paths

Search for any workflow files or scripts that reference the old paths:

```bash
grep -r "npm_publishing_setup" .github/ scripts/ Makefile
grep -r "sdk_unit_tests"       .github/ scripts/ Makefile
```

Update any references to point to the new locations.

### 6. Update SDK README

Add a note to `sdk/typescript/README.md` explaining the directory layout:

```markdown
## Repository Layout

| Directory | Purpose |
|---|---|
| `src/` | Runtime source — compiled and published to npm |
| `scripts/` | Build and publishing helpers — not published |
| `tests/` | Unit and integration tests — not published |
| `generated/` | Auto-generated bindings — do not edit manually |
```

---

## Acceptance Criteria

- [ ] `npm pack --dry-run` produces a tarball that contains **only** runtime code under `dist/`.
- [ ] `npm_publishing_setup.ts` and `sdk_unit_tests.ts` do **not** appear in the tarball.
- [ ] `npm run build` succeeds with no TypeScript errors.
- [ ] `npm test` passes (all existing tests continue to run from their new location).
- [ ] CI is green (lint, type-check, tests).
- [ ] No new TODOs without a linked issue.

---

## Runtime Compatibility

This change is entirely structural — no public API changes. The published `dist/` output is unaffected as long as `npm_publishing_setup.ts` and `sdk_unit_tests.ts` are not imported by any runtime source file in `src/`.

Before moving, confirm this with:

```bash
grep -r "npm_publishing_setup\|sdk_unit_tests" src/
```

If any runtime file imports either of these, that import is a bug that should be removed as part of this change.

---

## Cross-Environment Note

The SDK must continue to work in Node.js, browsers, and React Native. Moving these files does not affect that constraint since they are non-runtime. Verify `src/react_native_compatibility.ts` and the `browser` / `exports` fields in `package.json` are unchanged.

---

## References

- [npm `files` field documentation](https://docs.npmjs.com/cli/v10/configuring-npm/package-json#files)
- [sdk/typescript/README.md](../sdk/typescript/README.md)
- [sdk/typescript/package.json](../sdk/typescript/package.json)
- [DEPENDENCIES.md](./DEPENDENCIES.md)
