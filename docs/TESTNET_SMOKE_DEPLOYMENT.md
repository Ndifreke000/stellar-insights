# Testnet Smoke Test Deployment Guide

## Issue #1843: Get a Live Backend Instance for Testnet-Smoke Tests

### Prerequisites

The `testnet-smoke` test suite (`backend/tests/testnet/smoke_test.rs`) requires
a live, deployed backend instance to exercise real HTTP and WebSocket endpoints.

### Required Environment Variables

| Variable | Description | Where to Set |
|----------|-------------|--------------|
| `TESTNET_API_URL` | Base URL of the deployed testnet backend | CI secrets or local env |
| `TESTNET_API_KEY` | API key for authenticated endpoints | CI secrets or local env |

### Deployment Path

1. **Deploy via CI**: The `deploy-testnet.yml` workflow handles ECR push and
   K8s deployment. Ensure the pre-flight validation passes (see #1844).

2. **Verify the deployment**: Once deployed, the backend URL is available
   as the `TESTNET_BACKEND_URL` repository variable.

3. **Run the smoke tests**:
   ```bash
   cd backend
   export TESTNET_API_URL="https://testnet.stellar-insights.example.com"
   export TESTNET_API_KEY="your-api-key"
   cargo test --features testnet-smoke --test testnet_smoke
   ```

4. **In CI**: The `deploy-testnet.yml` workflow already runs the smoke tests
   as a post-deployment step with the required env vars configured.

### First Run Expectations

This suite has never been exercised against a live instance. Treat a
first-run failure as expected/likely — fix issues as they surface rather
than treating them as regressions.

### Test Coverage

The smoke tests exercise:
- Health endpoint (`/health`)
- Authentication (`/api/v1/auth`)
- Transaction endpoints (`/api/v1/transactions`)
- Analytics endpoints (`/api/v1/analytics`)
- WebSocket connections
