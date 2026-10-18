# Mainnet Incident Response Runbook

This guide provides a step-by-step procedure for responding to incidents on the PayRaider mainnet.

## 1. Alert Triage
When a PagerDuty alert fires, first:
- Check Grafana dashboard at `https://grafana.example.com/d/payraider`
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
- **Symptoms**: `db_pool_utilization` at 100%, `db_pool_errors_total` increasing.
  Check both the read pool and the write pool (`write_pool_metrics()` --
  write-pool exhaustion is the more likely story given SQLite's
  single-writer constraint; see ADR 0001)
- **Steps**:
  1. Check pool metrics in Grafana (read pool vs. write pool separately)
  2. If it's the write pool: this is expected under sustained write load,
     not a bug -- see ADR 0001's "Revisit this decision when..." triggers
  3. If it's the read pool: identify what's holding connections open
     (slow queries logged via `log_explain_query_plan`, see database.rs)
  4. Consider temporarily raising DB_POOL_MAX_CONNECTIONS (read) or
     DB_WRITE_POOL_MAX_CONNECTIONS (write, capped by SQLite's actual
     single-writer limit -- raising this doesn't add real throughput)
  5. Investigate root cause of high load

### 2.3 Contract Pause Procedure
To pause the payraider contract:
1. Connect to the contract admin account
2. Execute `payraider.pause()`
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
