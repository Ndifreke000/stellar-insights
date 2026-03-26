# 🚨 URGENT: CI Dependency Issue - Quick Fix Guide

## The Problem

CI is failing with:
```
error: no matching package named `opentelemetry-attribute-utils` found
required by package `stellar-insights-apm v0.1.0 (/home/runner/work/stellar-insights/stellar-insights/backend/apm)`
```

## Why This Is Happening

The CI is trying to build a package (`stellar-insights-apm`) that **doesn't exist** in your current codebase. This means:

1. **Old commit**: CI might be building an old commit that had this package
2. **Wrong branch**: CI might be checking out the wrong branch
3. **Stale cache**: CI cache might have old dependency information
4. **Merge conflict**: There might be unmerged changes from another branch

## 🔥 Quick Fix (Do This First)

### Step 1: Check What Branch CI Is Building

Look at the CI logs and find the line that says:
```
Checking out <branch-name> at commit <hash>
```

Make sure it matches your current branch.

### Step 2: Force Regenerate Cargo.lock

```bash
cd backend
rm -f Cargo.lock
cargo build
```

If this succeeds locally, commit and push:
```bash
git add Cargo.lock
git commit -m "fix: regenerate Cargo.lock to resolve CI dependency issue"
git push
```

### Step 3: Clear CI Cache

Add this to your PR or push an empty commit to trigger a fresh build:
```bash
git commit --allow-empty -m "chore: trigger CI rebuild with fresh cache"
git push
```

## 🔍 Diagnostic Steps

### Check 1: Verify No APM Package Exists

```bash
# Should return nothing
find . -type d -name "apm"

# Should return nothing
grep -r "stellar-insights-apm" backend/Cargo.toml
```

### Check 2: Verify Current Dependencies

```bash
cd backend
cargo tree 2>&1 | grep -i "apm\|opentelemetry-attribute"
```

If you see any references to `apm` or `opentelemetry-attribute-utils`, there's a problem.

### Check 3: Check Git History

```bash
# See if apm was recently removed
git log --all --full-history --oneline -- "**/apm/**"

# Check recent Cargo.toml changes
git log --oneline -10 -- backend/Cargo.toml
```

## 🛠️ Detailed Solutions

### Solution A: The Package Was Recently Removed

If `apm` was recently removed from the codebase:

1. **Ensure complete removal:**
   ```bash
   # Remove any lingering references
   git rm -rf backend/apm 2>/dev/null || true
   
   # Regenerate lock file
   cd backend
   rm Cargo.lock
   cargo build
   
   # Commit
   git add -A
   git commit -m "fix: complete removal of apm package and regenerate Cargo.lock"
   git push
   ```

### Solution B: Workspace Configuration Issue

If there's a workspace configuration somewhere:

1. **Find all Cargo.toml files:**
   ```bash
   find . -name "Cargo.toml" -type f
   ```

2. **Check each for workspace or apm references:**
   ```bash
   find . -name "Cargo.toml" -exec grep -l "apm\|workspace" {} \;
   ```

3. **Remove any apm references from workspace members**

### Solution C: CI Is Building Wrong Commit

1. **Check your PR:**
   - Ensure you're pushing to the correct branch
   - Verify the PR is comparing the right branches
   - Check if there are merge conflicts

2. **Rebase if needed:**
   ```bash
   git fetch origin
   git rebase origin/main  # or origin/develop
   git push --force-with-lease
   ```

### Solution D: Nuclear Option (Fresh Start)

If nothing else works:

```bash
# 1. Clean everything locally
cd backend
cargo clean
rm -rf target
rm Cargo.lock

# 2. Rebuild from scratch
cargo build --all-targets --all-features

# 3. If successful, commit
git add Cargo.lock
git commit -m "fix: fresh Cargo.lock after clean build"

# 4. Force push to trigger fresh CI
git push --force-with-lease
```

## 📋 CI Workflow Fix

If the issue is in the CI workflow itself, update `.github/workflows/backend.yml`:

```yaml
- name: Clean cargo state
  working-directory: ./backend
  run: |
    cargo clean
    rm -f Cargo.lock

- name: Generate fresh Cargo.lock
  working-directory: ./backend
  run: cargo generate-lockfile

- name: Build
  working-directory: ./backend
  run: cargo build --verbose
```

## ✅ Verification

After applying the fix, verify:

1. **Local build works:**
   ```bash
   cd backend
   cargo clean
   cargo build --all-targets --all-features
   cargo clippy --all-targets --all-features -- -D warnings
   ```

2. **No apm references:**
   ```bash
   cargo tree | grep -i apm
   # Should return nothing
   ```

3. **CI passes:**
   - Push your changes
   - Watch the CI build
   - Verify it completes successfully

## 🎯 Most Likely Cause

Based on the error, the most likely cause is:

**The CI is building from a commit that still had the `apm` package, or there's a stale Cargo.lock in the CI cache.**

**Recommended immediate action:**
```bash
cd backend
rm Cargo.lock
cargo build
git add Cargo.lock
git commit -m "fix: regenerate Cargo.lock"
git push
```

This should resolve the issue in 90% of cases.

## 📞 Still Not Working?

If the issue persists:

1. **Check the exact commit CI is building:**
   - Look at CI logs for the commit hash
   - Checkout that commit locally: `git checkout <hash>`
   - See if the apm directory exists there

2. **Check for branch protection rules:**
   - Ensure your branch can be pushed to
   - Verify CI is configured to build your branch

3. **Contact DevOps:**
   - There might be a CI configuration issue
   - Cache might need manual clearing
   - Workflow might need updating

## 🔐 Prevention

To prevent this in the future:

1. **Always commit Cargo.lock** for binary projects
2. **Clean CI caches** when removing packages
3. **Use `cargo clean`** before major changes
4. **Test locally** before pushing
5. **Keep dependencies up to date**

---

**Status**: 🚨 Blocking CI  
**Priority**: 🔥 Critical  
**Action Required**: Regenerate Cargo.lock and push  
**ETA**: 5 minutes
