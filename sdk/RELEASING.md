# Releasing the SDKs

The SDKs are versioned with [Changesets](https://github.com/changesets/changesets)
and published by [`.github/workflows/sdk-release.yml`](../.github/workflows/sdk-release.yml).
Nobody runs `npm publish` by hand.

| Package | Directory | Registry |
| --- | --- | --- |
| `@payraider/sdk` | `typescript/` | npm |
| `@payraider/react` | `react/` | npm |
| `@payraider/mcp-server` | `mcp-server/` | npm |
| `payraider` | `python/` | PyPI (version mirrored in `python/package.json`) |

## 1. In your PR: add a changeset

```bash
cd sdk
npm ci
npx changeset
```

Pick the packages you changed and the bump (`patch` for fixes, `minor` for
additions, `major` for breaking changes, see [COMPATIBILITY.md](COMPATIBILITY.md#versioning-policy)).
Then write the one-line summary that should appear in the changelog. Commit
the generated `.changeset/*.md` file. For a breaking change, also describe the
migration in the summary.

The PR check runs `npm run compat:check` and reports which packages have
pending changesets.

## 2. After merge: the Version Packages PR

On every push to `main` that touches `sdk/**`, the workflow opens or updates
a PR titled **"chore(sdk): version packages"**. It runs
`npm run version-packages`, which does three things:

1. `changeset version` bumps the versions and writes each package's `CHANGELOG.md`.
2. `scripts/release/sync-python-version.mjs` copies the Python version into `python/pyproject.toml`.
3. `scripts/release/update-compatibility.mjs` adds a row to `compatibility.json`
   and regenerates the matrix in `COMPATIBILITY.md`.

Review it like any PR. Edit `compatibility.json` there if a row needs a
`notes` or `networks` change, then re-run
`node scripts/release/update-compatibility.mjs`.

## 3. Merge it: publish

Merging the Version Packages PR triggers the publish pass:

- `scripts/release/publish.mjs` publishes each npm package whose current
  version is not on npm yet. It runs `npm ci && npm run build && npm publish`
  inside that package (with provenance) and prints `New tag:` lines, which
  the action turns into git tags and GitHub releases.
- The `python` job builds `sdk/python` and uploads it to PyPI with trusted
  publishing (`skip-existing`, so re-runs are harmless).

## Setup (one-time)

- Add the `NPM_TOKEN` repository secret: an npm automation token with publish
  rights on the `@payraider` scope.
- On PyPI, add a trusted publisher for this repo, workflow `sdk-release.yml`,
  environment `pypi`. Create the `pypi` environment in the repo settings.
- In repository settings → Actions, allow GitHub Actions to create pull requests.

## Troubleshooting

| Symptom | Fix |
| --- | --- |
| `compat:check` fails on a PR | Versions were bumped without regenerating the matrix. Run `cd sdk && node scripts/release/update-compatibility.mjs` and commit |
| Publish step logs `skip … already on npm` | Expected for packages that didn't change |
| A publish failed halfway | Re-run the workflow. Already-published versions are skipped |
| Need to see what would publish | `cd sdk && node scripts/release/publish.mjs --dry-run` |
| Changesets warns about a `file:../typescript` dependency | Expected. Local `file:` links are ignored when bumping dependents. The published peer range (`^1.0.0`) is what consumers see |

## Why not npm workspaces?

Each SDK keeps its own `package-lock.json`, and existing CI runs `npm ci` per
directory. Making `sdk/` an npm workspace root would change that for every
job. Instead, `sdk/pnpm-workspace.yaml` tells changesets where the packages
are, and npm ignores it. pnpm itself is never used.
