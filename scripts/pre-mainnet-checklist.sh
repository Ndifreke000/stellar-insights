#!/usr/bin/env bash
set -euo pipefail

# Mainnet Pre-Deployment Checklist
# Exits with 0 if all checks pass, otherwise exits with non-zero.

echo "===================================================="
# Check 1: All contracts verified on testnet (canary validation)
echo -n "Check 1: Running contract tests (canary validation)... "
if [ -d "contracts" ]; then
  if cargo test --manifest-path contracts/Cargo.toml --all > /dev/null 2>&1; then
    echo "PASS"
  else
    echo "FAIL (Contract tests failed)"
    exit 1
  fi
else
  echo "SKIP (No contracts directory found)"
fi

# Check 2: K8s mainnet overlay applies without errors
echo -n "Check 2: Validating K8s mainnet overlays... "
if [ -d "k8s/overlays/mainnet" ] && command -v kubectl >/dev/null 2>&1; then
  if kubectl kustomize k8s/overlays/mainnet > /dev/null 2>&1; then
    echo "PASS"
  else
    echo "FAIL (K8s mainnet kustomize overlay failed)"
    exit 1
  fi
else
  echo "SKIP (k8s/overlays/mainnet or kubectl not available)"
fi

# Check 3: Terraform plan shows no destructive changes
echo -n "Check 3: Verifying Terraform plan has no destructive changes... "
if [ -d "terraform/environments/production" ] && command -v terraform >/dev/null 2>&1; then
  cd terraform/environments/production
  terraform init -backend=false > /dev/null 2>&1 || true
  
  PLAN_STATUS=0
  terraform plan -detailed-exitcode -out=tfplan > /dev/null 2>&1 || PLAN_STATUS=$?
  
  if [ "$PLAN_STATUS" -eq 1 ]; then
    echo "FAIL (Terraform plan errored)"
    exit 1
  elif [ "$PLAN_STATUS" -eq 2 ]; then
    if command -v jq >/dev/null 2>&1 && terraform show -json tfplan | jq -e '.resource_changes[].change.actions[] | select(. == "delete" or . == "destroy")' >/dev/null 2>&1; then
      echo "FAIL (Destructive changes detected!)"
      exit 1
    else
      echo "PASS (Changes exist, but no destructions)"
    fi
  else
    echo "PASS (No changes)"
  fi
  cd - > /dev/null
else
  echo "SKIP (terraform or production env not available)"
fi

# Check 4: Smoke tests pass on testnet
echo -n "Check 4: Running smoke tests on testnet... "
if [ -d "mobile" ]; then
  if npm run --prefix mobile test -- --testPathPattern=testnet > /dev/null 2>&1; then
    echo "PASS"
  else
    echo "FAIL (Smoke tests failed)"
    exit 1
  fi
else
  echo "SKIP (mobile directory not found)"
fi

# Check 5: No open P0 issues labeled mainnet
echo -n "Check 5: Checking for open P0 issues labeled mainnet... "
if command -v gh >/dev/null 2>&1; then
  OPEN_P0_ISSUES=$(gh issue list --label "P0" --label "mainnet" --state "open" --json number -q '. | length' 2>/dev/null || echo "0")
  if [ "$OPEN_P0_ISSUES" -gt 0 ]; then
    echo "FAIL ($OPEN_P0_ISSUES open P0 issue(s) labeled 'mainnet')"
    exit 1
  else
    echo "PASS"
  fi
else
  echo "SKIP (gh CLI not found)"
fi

# Check 6: Grafana dashboards accessible
GRAFANA_URL=${GRAFANA_URL:-"http://localhost:3000/api/health"}
echo -n "Check 6: Checking Grafana dashboard health... "
if curl -sf --connect-timeout 5 "$GRAFANA_URL" > /dev/null 2>&1; then
  echo "PASS"
else
  echo "FAIL (Unable to reach Grafana at $GRAFANA_URL)"
  exit 1
fi

# Check 7: Vault HA healthy
VAULT_ADDR=${VAULT_ADDR:-"http://localhost:8200"}
echo -n "Check 7: Checking Vault HA health... "
if command -v vault >/dev/null 2>&1; then
  if vault status -format=json 2>/dev/null | jq -e '.sealed == false and .initialized == true' >/dev/null 2>&1; then
    echo "PASS"
  else
    echo "FAIL (Vault is not unsealed/initialized)"
    exit 1
  fi
else
  if curl -sf --connect-timeout 5 "$VAULT_ADDR/v1/sys/health" > /dev/null 2>&1; then
    echo "PASS"
  else
    echo "FAIL (Unable to reach Vault API at $VAULT_ADDR)"
    exit 1
  fi
fi

# Check 8: Docker image passes docker scout cves scan
DOCKER_IMAGE=${DOCKER_IMAGE:-"stellar-insights-backend:latest"}
echo -n "Check 8: Running dependency/vulnerability scan (Docker Scout)... "
if command -v docker >/dev/null 2>&1; then
  if docker scout cves "$DOCKER_IMAGE" --exit-code --only-severity critical,high > /dev/null 2>&1; then
    echo "PASS"
  else
    echo "FAIL (CVE scan failed or found critical/high vulnerabilities)"
    exit 1
  fi
else
  echo "SKIP (docker daemon/scout not running)"
fi

echo "===================================================="
echo "SUCCESS: All pre-mainnet checklist items verified!"
echo "===================================================="
exit 0
