# Mainnet Deployment Runbook

This runbook outlines the step-by-step procedure for deploying PayRaider smart contracts, backend services, and frontend to the Stellar mainnet environment.

---

## 1. Pre-Deployment Checklist

Before initiating any mainnet deployment, you must verify the readiness of the system by executing the automated pre-mainnet checklist script.

### Automated Verification
Run the following script from the root directory of the repository:
```bash
./scripts/pre-mainnet-checklist.sh
```

This script will verify the following criteria:
1. All smart contracts successfully build and pass test suites (canary validation).
2. Kubernetes mainnet overlays apply without errors: `kubectl kustomize k8s/overlays/production`.
3. Terraform plan shows no destructive changes: `terraform plan` in `terraform/environments/production`.
4. Smoke tests pass on testnet: `npm run --prefix mobile test -- --testPathPattern=testnet`.
5. No open P0 GitHub issues are labeled `mainnet`.
6. Grafana dashboards and monitoring tools are fully accessible.
7. Vault HA instance is unsealed, initialized, and healthy.
8. Docker image passes dependency and vulnerability scans via `docker scout cves`.

> [!WARNING]
> If any checklist item fails, the script will exit with a non-zero exit code. You **must not** proceed with mainnet deployment until the blocker is resolved.

---

## 2. Contract Deployment

Deploying Soroban smart contracts to the Stellar mainnet is handled via the deployment scripts located in the `contracts/` and `scripts/` directories.

### Step 1: Set Up Environment Variables
Ensure your environment variables are configured for mainnet access:
```bash
export STELLAR_NETWORK="mainnet"
export SOROBAN_RPC_URL="https://soroban-rpc.mainnet.stellar.org" # or your custom mainnet RPC endpoint
export DEPLOYER_SECRET_KEY="your-encrypted-vault-secret"
```

### Step 2: Deploy Contracts
Run the contract deploy script pointing to mainnet:
```bash
# Compile and deploy all contracts, then update state snapshot configs
./scripts/deploy-contracts.sh mainnet
```
*Note: This script registers the contract WASM files on-chain, initializes the contracts (governance, access-control, escrow, analytics), and outputs the contract IDs to the production configurations.*

---

## 3. Infrastructure Deployment

Infrastructure changes are managed via Terraform and deployed using the mainnet CI workflow.

### Continuous Integration Workflow
1. Push changes to the `main` branch or manually trigger the deployment workflow via the GitHub Actions UI.
2. The workflow [Deploy Backend (Blue-Green)](file://../../.github/workflows/deploy.yml) will initiate.
3. Select `production` as the target environment when triggering manually.
4. The workflow will perform the following:
   - Configure AWS credentials using the configured OIDC role.
   - Build and push the Docker image to Amazon ECR.
   - Register a new ECS Task Definition.
   - Create a CodeDeploy blue-green deployment to update the containers on AWS ECS/Kubernetes with zero downtime.

---

## 4. Post-Deployment Verification

After the infrastructure deployment completes successfully, perform smoke testing and canary validation on mainnet to guarantee the environment's health.

### Step 1: Running Smoke Tests
Verify connectivity and simple read operations on the deployed API:
```bash
# Run testnet smoke tests as a baseline
npm run --prefix mobile test -- --testPathPattern=testnet

# Run mainnet endpoint check
curl -sf "https://api.payraider.com/health"
```

### Step 2: Canary Validation
1. Monitor logs using Grafana/Loki to verify there are no 5xx errors or uncaught exceptions.
2. Confirm the mobile app connects properly when switched to `mainnet`.

---

## 5. Rollback Procedure

In the event of an outage or a failed verification post-deployment, follow these instructions to roll back changes immediately.

### Contract Rollback
If a contract deployment went wrong, use the upgrade contract to update the WASM hash to the previous stable state:
```bash
./scripts/rollback-contracts.sh mainnet
```

### Infrastructure & Backend Rollback
Trigger the rollback workflow [Rollback Backend](file://../../.github/workflows/rollback.yml) or manually run the rollback script:
```bash
./scripts/rollback.sh production
```
This forces AWS CodeDeploy or Kubernetes to switch traffic back to the previous stable blue task definition or deployment replica.
