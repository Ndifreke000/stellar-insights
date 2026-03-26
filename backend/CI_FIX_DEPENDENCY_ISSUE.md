# CI Dependency Issue Fix

## Problem

The CI is failing with:
```
error: no matching package named `opentelemetry-attribute-utils` found
location searched: crates.io index
required by package `stellar-insights-apm v0.1.0 (/home/runner/work/stellar-insights/stellar-insights/backend/apm)`
```

## Root Cause

The error indicates the CI is trying to build a package `stellar-insights-apm` from `backend/apm` directory, but:
1. This directory doesn't exist in the current codebase
2. This package is not referenced in `backend/Cargo.toml`
3. The dependency `opentelemetry-attribute-utils` doesn't exist on crates.io

This suggests the CI might be:
- Running on an old commit/branch
- Using a cached Cargo.lock file
- Building from a different branch than expected

## Solutions

### Solution 1: Regenerate Cargo.lock (Recommended)

```bash
cd backend
rm Cargo.lock
cargo build
git add Cargo.lock
git commit -m "chore: regenerate Cargo.lock to fix dependency resolution"
```

### Solution 2: Check CI Branch

Ensure the CI is running on the correct branch:
1. Check the PR branch name
2. Verify the CI workflow is checking out the correct branch
3. Check if there are any workspace configurations at the root level

### Solution 3: Clean CI Cache

If the issue persists, the CI cache might be stale:

Add this step to your CI workflow before building:
```yaml
- name: Clean cargo cache
  run: |
    cargo clean
    rm -rf ~/.cargo/registry/index/*
    rm -rf ~/.cargo/registry/cache/*
```

### Solution 4: Check for Workspace Configuration

If there's a root-level `Cargo.toml` with workspace configuration, ensure it doesn't reference the `apm` package:

```toml
[workspace]
members = [
    "backend",
    # "backend/apm",  # Remove this if present
]
```

## Verification Steps

After applying the fix:

1. **Local verification:**
   ```bash
   cd backend
   cargo clean
   cargo build --all-targets --all-features
   cargo clippy --all-targets --all-features -- -D warnings
   ```

2. **Check dependencies:**
   ```bash
   cargo tree | grep -i apm
   cargo tree | grep -i opentelemetry
   ```

3. **Verify no workspace issues:**
   ```bash
   # From root directory
   find . -name "Cargo.toml" -exec grep -l "workspace" {} \;
   ```

## Current Status

Based on the current codebase analysis:
- ✅ `backend/Cargo.toml` does NOT reference `stellar-insights-apm`
- ✅ No `backend/apm` directory exists
- ✅ No workspace configuration in `backend/Cargo.toml`
- ❌ CI is somehow trying to build a non-existent package

## Recommended Action

**Immediate fix:**
```bash
# 1. Ensure you're on the correct branch
git status

# 2. Regenerate Cargo.lock
cd backend
rm Cargo.lock
cargo build

# 3. Commit the new Cargo.lock
git add Cargo.lock
git commit -m "fix: regenerate Cargo.lock to resolve dependency issues"
git push
```

**If that doesn't work:**
```bash
# Check if there's a root workspace configuration
cat ../Cargo.toml 2>/dev/null

# If it exists and references apm, remove that reference
# Then regenerate all lock files
```

## Prevention

To prevent this issue in the future:

1. **Always commit Cargo.lock:**
   - Cargo.lock should be committed for binary projects
   - Ensures consistent builds across environments

2. **Clean builds in CI:**
   - Use fresh caches or clean before building
   - Don't rely on stale cached dependencies

3. **Workspace management:**
   - Keep workspace configurations up to date
   - Remove references to deleted packages

4. **Dependency audits:**
   - Regularly run `cargo tree` to check dependencies
   - Use `cargo-outdated` to check for updates

## Additional Information

### About opentelemetry-attribute-utils

This crate doesn't exist on crates.io. If it was a custom internal crate, it should either:
- Be published to crates.io
- Be included as a path dependency
- Be removed if no longer needed

### About stellar-insights-apm

If this was an APM (Application Performance Monitoring) package that's no longer needed:
1. Remove all references from workspace configurations
2. Delete the directory if it still exists
3. Regenerate Cargo.lock
4. Update documentation

## Contact

If the issue persists after trying these solutions:
1. Check the CI logs for the exact commit being built
2. Verify the branch being tested
3. Check for any custom CI scripts that might be modifying Cargo.toml
4. Review recent commits that might have introduced workspace changes
