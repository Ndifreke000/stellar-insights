# Branch Protection Policy for Stellar Insights

This document defines the branch protection rules for the `main` branch to ensure code quality and prevent regressions now that CI is reliably green.

## Background

Prior to this stabilization pass, the backend had:
- 26 cargo check compile errors
- Tests never passing
- Axum route-syntax bug preventing server boot

With these issues fixed, CI is now reliably green. This policy prevents silent drift back into a broken state.

## Protected Branch: `main`

### Required Status Checks

The following GitHub Actions workflows **must pass** before any PR can merge to `main`:

| Workflow | Job Name | Purpose | Scope |
|----------|----------|---------|-------|
| **Clippy** | `clippy` | Catch Rust warnings and code smells | `backend/**`, `contracts/**` |
| **Deploy Testnet** | `test` | Unit and integration tests for backend | `backend/**` |
| **Frontend Build** | `build` | Build Next.js frontend; catch build errors | `frontend/**` |
| **Frontend Lint** | (to be created) | ESLint and TypeScript type checks | `frontend/**` |

### Required Checks Configuration

```yaml
# Branch protection for 'main'
required_status_checks:
  strict: true  # Require branch to be up-to-date before merging
  contexts:
    - "Clippy"                    # workflow name
    - "Deploy Testnet / test"     # workflow name / job name
    - "Frontend Build / Build & Metrics"  # workflow name / job name
    - "Frontend Lint / lint"      # (once workflow is created)

# Additional protections
require_code_review: true
required_approving_review_count: 1
require_status_checks_to_pass_before_merge: true
restrict_who_can_push_to_matching_branches: false  # Allow admins to bypass
```

## Workflow Status

### ✅ Clippy (Backend Code Quality)
- **Workflow:** `.github/workflows/clippy.yml`
- **Job:** `clippy`
- **Status:** Green ✅
- **What it checks:**
  - Backend Clippy lints
  - Contracts Clippy lints (continue-on-error; doesn't block)
- **Path filtering:** Automatic (configured in workflow)

### ✅ Deploy Testnet (Backend Tests)
- **Workflow:** `.github/workflows/deploy-testnet.yml`
- **Job:** `test` (Unit & Integration Tests)
- **Status:** Green ✅
- **What it checks:**
  - Backend unit tests: `cargo test --features sep-integration`
  - Database schema via SQL migrations
- **Path filtering:** `backend/**`, `k8s/**`, `.github/workflows/deploy-testnet.yml`
- **Environment:** SQLite in-memory database, JWT_SECRET set

### ✅ Frontend Build (Frontend Build Success)
- **Workflow:** `.github/workflows/frontend-build.yml`
- **Job:** `build` (Build & Metrics)
- **Status:** Green ✅
- **What it checks:**
  - Next.js build succeeds
  - Build time under 2 minutes (warning if over)
- **Path filtering:** `frontend/**`
- **Note:** Does NOT include linting; see next section

### ⚠️ Frontend Lint (Frontend Code Quality)
- **Workflow:** NOT YET CREATED
- **What it should check:**
  - `npm run lint` (ESLint)
  - `tsc --noEmit` (TypeScript type checking)
- **Path filtering:** `frontend/**`
- **Status:** Needs to be created

## Creating the Frontend Lint Workflow

The issue #1885 specifies: "frontend npm run lint (or tsc --noEmit at minimum)"

Create `.github/workflows/frontend-lint.yml`:

```yaml
name: Frontend Lint

on:
  push:
    branches: [main]
    paths:
      - 'frontend/**'
  pull_request:
    paths:
      - 'frontend/**'

jobs:
  lint:
    name: Lint & Type Check
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: frontend

    steps:
      - uses: actions/checkout@v4

      - uses: pnpm/action-setup@v4
        with:
          version: 10

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: frontend/pnpm-lock.yaml

      - run: pnpm install --frozen-lockfile

      - name: ESLint
        run: npm run lint

      - name: TypeScript Type Check
        run: npm run typecheck || npx tsc --noEmit
```

### Verify Frontend Lint is Green

Before enabling in branch protection:

1. **Manually run the lint checks:**
   ```bash
   cd frontend
   npm run lint
   npx tsc --noEmit
   ```

2. **Commit and push to a test branch** to verify workflow passes in CI

3. **Once confirmed green**, add to branch protection required checks

## Configuring Branch Protection

### Option A: GitHub Web UI

1. Go to: https://github.com/Ndifreke000/stellar-insights/settings/branches
2. Click "Add rule" under "Branch protection rules"
3. Branch name pattern: `main`
4. Enable:
   - [x] Require status checks to pass before merging
   - [x] Require branches to be up to date before merging
   - [x] Require a pull request before merging
   - [x] Require approvals (1)
   - [x] Dismiss stale pull request approvals when new commits are pushed
5. Under "Status checks that are required":
   - [ ] Clippy
   - [ ] Deploy Testnet / test
   - [ ] Frontend Build / Build & Metrics
   - [ ] Frontend Lint / lint
6. Click "Create"

### Option B: GitHub CLI

```bash
# Requires: gh cli + admin permissions to repo

# Enable required status checks
gh api repos/Ndifreke000/stellar-insights/branches/main/protection \
  --input - << 'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["Clippy", "Deploy Testnet / test", "Frontend Build / Build & Metrics", "Frontend Lint / lint"]
  },
  "required_pull_request_reviews": {
    "required_approving_review_count": 1,
    "dismiss_stale_reviews": true
  },
  "enforce_admins": false,
  "restrictions": null
}
EOF
```

### Option C: Terraform (Infrastructure as Code)

If repo is managed by Terraform:

```hcl
resource "github_branch_protection" "main" {
  repository_id = data.github_repository.stellar_insights.node_id
  pattern       = "main"

  required_status_checks {
    strict   = true
    contexts = [
      "Clippy",
      "Deploy Testnet / test",
      "Frontend Build / Build & Metrics",
      "Frontend Lint / lint"
    ]
  }

  required_pull_request_reviews {
    required_approving_review_count = 1
    dismiss_stale_reviews          = true
  }

  enforce_admins = false
}
```

## Verifying Configuration

After enabling branch protection:

```bash
# Check that protection is enabled
gh api repos/Ndifreke000/stellar-insights/branches/main/protection

# Create a test PR that intentionally fails a check
# (e.g., add a clippy warning)
# Verify that:
# 1. CI runs and reports status
# 2. Merge button is disabled until all checks pass
# 3. Dismissing a check shows "Status checks failed" message
```

## Special Cases

### Hot Fixes or Emergency Bypasses

If a critical fix needs to bypass branch protection:

1. **Minimum bypass:** One admin approval + manually dismiss status check failure
   - Not recommended for regular use
   - Document reason in PR description for audit trail

2. **Better approach:** Fix branch protection issue (make check pass) rather than bypass
   - If a workflow is flaky: fix root cause or adjust timeout
   - If a workflow is unnecessary: remove from required checks

### Path-Filtered Workflows

Some workflows use path filtering to only run when certain files change:

| Workflow | Paths | Note |
|----------|-------|------|
| Deploy Testnet | `backend/**`, `k8s/**` | Won't run if only frontend changes |
| Frontend Build | `frontend/**` | Won't run if only backend changes |

**GitHub branch protection behavior:** If a required check is not run (due to path filtering), it's automatically marked as passing. This is correct; we don't want to require frontend lints when only backend changes.

## Maintenance

### Quarterly Review

At the end of each quarter, audit:
- Are all required checks actually passing?
- What's the merge failure rate due to checks?
- Are there flaky checks that need investigation?
- Has the team composition changed (adjust reviewer count)?

### Adding New Required Checks

When a new workflow is ready:

1. Verify it's green for 1 week on all recent commits
2. Update this document with workflow details
3. Configure in branch protection
4. Announce to team in #stellar-insights-dev

### Removing Required Checks

If a check is permanently removed:

1. Document reason and date in this file (git log)
2. Remove from branch protection
3. Delete or archive the workflow

## Emergency Procedures

### If Branch Protection Blocks All Merges

This can happen if:
- All CI workflows are failing
- A required workflow is deleted but still marked as required in protection settings

**Recovery:**

1. **If CI is broken:** Fix the underlying issue (revert bad commit, fix workflow)
2. **If workflow is missing:** GitHub automatically removes the check after ~30 days; admin can manually remove sooner
3. **As last resort:** Temporarily disable branch protection, merge fix, re-enable

## Related Documentation

- **Monitoring & SLOs:** `docs/SLOs_AND_ALERTING.md` — alerts for deployment failures
- **Incident Response:** `docs/runbooks/mainnet-incident-response.md`
- **GitHub documentation:** https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches
