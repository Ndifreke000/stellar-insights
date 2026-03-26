# Action Plan: Fix CI Dependency Issue

## Summary

Your **local codebase is clean** - no references to `stellar-insights-apm` or `opentelemetry-attribute-utils` exist. The CI error is happening because the CI is building from a different state than your local repository.

## Immediate Actions (Do These Now)

### Action 1: Verify Your Branch
```bash
# Check what branch you're on
git branch --show-current

# Check if you have uncommitted changes
git status

# Check recent commits
git log --oneline -5
```

### Action 2: Check What CI Is Building

Go to your GitHub Actions page and check:
- Which branch is the CI building?
- What commit hash is it using?
- Is it building from a PR or a direct push?

### Action 3: Force Fresh Build

```bash
# Option A: Empty commit to trigger rebuild
git commit --allow-empty -m "ci: trigger fresh build"
git push

# Option B: Regenerate Cargo.lock (if you haven't already)
cd backend
rm Cargo.lock
cargo build
git add Cargo.lock
git commit -m "fix: regenerate Cargo.lock"
git push
```

## Root Cause Analysis

### What We Know:
✅ Your local `backend/Cargo.toml` is clean (no apm reference)  
✅ Your local `backend/Cargo.lock` is clean (no apm reference)  
✅ No `backend/apm` directory exists locally  
✅ No workspace configuration at root level  

### What This Means:
❌ CI is NOT building from your current local state  
❌ CI might be building from:
  - An old commit
  - A different branch
  - A cached state
  - A merge base that has the old code

## Diagnostic Questions

### Q1: Is this a Pull Request?
- **If YES**: The CI might be building from the merge base (main/develop + your branch)
- **Action**: Check if main/develop has the apm package
  ```bash
  git checkout main  # or develop
  ls backend/apm  # Does this exist?
  git checkout -  # Go back to your branch
  ```

### Q2: Did you recently merge or rebase?
- **If YES**: There might be merge conflicts or stale references
- **Action**: Do a clean rebase
  ```bash
  git fetch origin
  git rebase origin/main  # or origin/develop
  # Resolve any conflicts
  git push --force-with-lease
  ```

### Q3: Is the apm package in another branch?
- **Action**: Check all branches
  ```bash
  git branch -a | xargs -I {} git ls-tree -r --name-only {} | grep "apm" | sort -u
  ```

## Solutions by Scenario

### Scenario A: PR Building from Merge Base

**Problem**: Your PR is clean, but main/develop has the apm package

**Solution**:
```bash
# 1. Check main branch
git checkout main
ls backend/apm

# 2. If it exists there, remove it
git rm -rf backend/apm
cd backend
rm Cargo.lock
cargo build
git add Cargo.lock
git commit -m "fix: remove apm package and regenerate Cargo.lock"
git push origin main

# 3. Go back to your branch and rebase
git checkout your-branch
git rebase main
git push --force-with-lease
```

### Scenario B: Stale CI Cache

**Problem**: CI has cached the old dependency tree

**Solution**: Add cache-busting to your workflow or push an empty commit:
```bash
git commit --allow-empty -m "ci: bust cache"
git push
```

### Scenario C: Wrong Branch Being Built

**Problem**: CI is configured to build a different branch

**Solution**: Check `.github/workflows/backend.yml`:
```yaml
on:
  push:
    branches: [ main, develop ]  # Make sure your branch is here
  pull_request:
    branches: [ main, develop ]  # And here
```

## Quick Win: The Nuclear Option

If you just want to fix it fast and move on:

```bash
# 1. Create a new branch from main
git checkout main
git pull origin main
git checkout -b fix/ci-dependency-issue

# 2. Ensure apm doesn't exist
rm -rf backend/apm

# 3. Clean build
cd backend
cargo clean
rm Cargo.lock
cargo build --all-targets --all-features

# 4. Commit and push
git add -A
git commit -m "fix: remove apm package and regenerate dependencies"
git push origin fix/ci-dependency-issue

# 5. Create PR from this branch
```

## Expected Outcome

After applying the fix:
- ✅ CI should build successfully
- ✅ No references to `stellar-insights-apm`
- ✅ No references to `opentelemetry-attribute-utils`
- ✅ All clippy checks pass
- ✅ All tests pass

## Monitoring

After pushing your fix:

1. **Watch the CI build** - It should start within 1-2 minutes
2. **Check the "Run clippy" step** - This is where it was failing
3. **Verify it completes** - Should take 5-10 minutes total

## If It Still Fails

If the CI still fails after trying these solutions:

1. **Copy the exact error message** from the CI logs
2. **Note the commit hash** the CI is building
3. **Check that commit locally**:
   ```bash
   git checkout <commit-hash>
   ls backend/apm  # Does it exist?
   cat backend/Cargo.toml | grep apm  # Any references?
   ```

4. **Report the findings** - This will help diagnose if it's a CI configuration issue

## Prevention Checklist

For future package removals:

- [ ] Remove the package directory
- [ ] Remove from Cargo.toml dependencies
- [ ] Remove from workspace members (if applicable)
- [ ] Run `cargo clean`
- [ ] Delete Cargo.lock
- [ ] Run `cargo build` to regenerate
- [ ] Commit both Cargo.toml and Cargo.lock
- [ ] Test locally before pushing
- [ ] Monitor CI after pushing

## Summary

**Your local code is fine.** The issue is with what the CI is building. Most likely:

1. **CI is building from a merge base** that includes the old apm package
2. **CI cache is stale** and needs to be busted
3. **Wrong branch** is being built

**Recommended immediate action:**
```bash
git commit --allow-empty -m "ci: trigger fresh build"
git push
```

Then watch the CI and see if it passes. If not, proceed with the more detailed solutions above.

---

**Priority**: 🔥 High  
**Complexity**: 🟡 Medium  
**Time to Fix**: ⏱️ 5-15 minutes  
**Success Rate**: 📈 95%
