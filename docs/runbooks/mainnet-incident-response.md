# Mainnet Incident Response Runbook

This guide provides a step-by-step procedure for responding to incidents on the Stellar Insights mainnet.

## 1. Alert Triage
When a PagerDuty alert fires, first:
- Check Grafana dashboard at `https://grafana.example.com/d/stellar-insights`
- Identify which service or metric is in alarm
- Verify if this is a false positive or real issue

## 2. Common Incident Playbooks

### 2.1 RPC/Horizon Outage
- **Symptoms**: `rpc_errors_total` spiking, `stellar_ledger_lag_seconds` increasing
- **Steps**:
  1. Check if primary RPC/Horizon endpoints are reachable
  2. Switch to backup endpoints in config
  3. Monitor `rpc_calls_total` and `rpc_call_duration_seconds`
  4. Notify team of endpoint switch

### 2.2 Database Pool Exhaustion
- **Symptoms**: `db_pool_utilization` at 100%, `db_pool_errors_total` increasing
- **Steps**:
  1. Check active connections in Grafana
  2. Identify long-running queries in PostgreSQL
  3. Kill slow/blocking queries if necessary
  4. Consider temporarily increasing pool size
  5. Investigate root cause of high load

### 2.3 Contract Pause Procedure
To pause the stellar_insights contract:
1. Connect to the contract admin account
2. Execute `stellar_insights.pause()`
3. Verify the contract is paused by checking state
4. Notify stakeholders

### 2.4 Rollback Procedure
To rollback to a previous deployment:
1. Ensure you're on the commit you want to rollback to
2. Run: `./scripts/rollback.sh <commit-hash>`
3. Verify services come up healthy
4. Monitor metrics for at least 15 minutes

## 3. Escalation Contacts
- Primary: [Name] ([email])
- Secondary: [Name] ([email])
- Engineering Manager: [Name] ([email])
