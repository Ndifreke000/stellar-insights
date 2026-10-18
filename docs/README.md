# PayRaider Documentation

> ⚠️ **This repository is deprecated.** See the [root README](../README.md#migration-checklist--archive-plan) for the migration plan and links to the Pay-Raider org where active development continues.

This index covers documentation that is still relevant to understanding or running this codebase. Docs for features outside the current off-ramp product scope have been moved to [`docs/archive/`](archive/).

---

## Core product docs

| Document | What it covers |
| --- | --- |
| [PLUGIN.md](PLUGIN.md) | Pre-payment check: REST API, TypeScript/Python SDKs, MCP server, Claude plugin |
| [API_DOCUMENTATION.md](API_DOCUMENTATION.md) | Full REST API reference |
| [DEPLOY.md](DEPLOY.md) | Deploying the API, plugin server, and web app on Render |
| [testnet-quickstart.md](testnet-quickstart.md) | Quick start on Stellar testnet |

## Architecture & design

| Document | What it covers |
| --- | --- |
| [architecture/ARCHITECTURE_ISSUES_AND_RECOMMENDATIONS.md](architecture/ARCHITECTURE_ISSUES_AND_RECOMMENDATIONS.md) | Known architectural gaps and recommendations |
| [architecture/API_VERSIONING.md](architecture/API_VERSIONING.md) | API versioning strategy |
| [adr/0001-sqlite-vs-postgres.md](adr/0001-sqlite-vs-postgres.md) | ADR: SQLite over Postgres |
| [RPC.md](RPC.md) | Stellar RPC client design |
| [patterns.md](patterns.md) | Code patterns used in the backend |
| [backend-modules.md](backend-modules.md) | Backend module map |
| [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) | Top-level directory layout |
| [DEPENDENCIES.md](DEPENDENCIES.md) | Key dependencies and why they were chosen |

## Operations & infrastructure

| Document | What it covers |
| --- | --- |
| [DEPLOY.md](DEPLOY.md) | Render deployment |
| [runbooks/mainnet-deployment.md](runbooks/mainnet-deployment.md) | Mainnet deploy runbook |
| [runbooks/mainnet-incident-response.md](runbooks/mainnet-incident-response.md) | Incident response |
| [runbooks/backup-restore.md](runbooks/backup-restore.md) | Backup and restore |
| [runbooks/credential-rotation.md](runbooks/credential-rotation.md) | Credential rotation |
| [DATABASE_MIGRATIONS.md](DATABASE_MIGRATIONS.md) | How migrations work |
| [backup-system.md](backup-system.md) | Backup system design |
| [disaster-recovery.md](disaster-recovery.md) | DR plan |
| [HORIZONTAL_SCALING.md](HORIZONTAL_SCALING.md) | Scaling notes |
| [BLUE_GREEN_DEPLOYMENT.md](BLUE_GREEN_DEPLOYMENT.md) | Blue/green deploy strategy |

## Observability & performance

| Document | What it covers |
| --- | --- |
| [PERFORMANCE.md](PERFORMANCE.md) | Performance targets and profiling notes |
| [API_SLOS.md](API_SLOS.md) | API SLOs |
| [SLOs_AND_ALERTING.md](SLOs_AND_ALERTING.md) | SLOs and alerting rules |
| [DEPLOYMENT_SLOS_AND_ALERTING.md](DEPLOYMENT_SLOS_AND_ALERTING.md) | Deployment-level SLOs |
| [APM_INTEGRATION.md](APM_INTEGRATION.md) | APM integration |
| [SENTRY_INTEGRATION.md](SENTRY_INTEGRATION.md) | Sentry error tracking |
| [ELK_LOG_AGGREGATION.md](ELK_LOG_AGGREGATION.md) | ELK log aggregation |
| [jaeger-tracing-guide.md](jaeger-tracing-guide.md) | Jaeger distributed tracing |
| [distributed-tracing-verification.md](distributed-tracing-verification.md) | Tracing verification |
| [prometheus-metrics.md](prometheus-metrics.md) | Prometheus metrics reference |
| [db-pool-metrics.md](db-pool-metrics.md) | DB pool metrics |
| [API_REQUEST_LOGGING.md](API_REQUEST_LOGGING.md) | Request logging |

## Security

| Document | What it covers |
| --- | --- |
| [security/ARCHITECTURE.md](security/ARCHITECTURE.md) | Security architecture |
| [security/THREAT_MODEL.md](security/THREAT_MODEL.md) | Threat model |
| [security/KNOWN_GAPS.md](security/KNOWN_GAPS.md) | Known security gaps |
| [security/SECURITY_TESTING_SCOPE.md](security/SECURITY_TESTING_SCOPE.md) | Security testing scope |
| [SECRETS_MANAGEMENT.md](SECRETS_MANAGEMENT.md) | Secrets management |
| [AUDIT_LOG.md](AUDIT_LOG.md) | Audit logging |
| [SESSION_MANAGEMENT.md](SESSION_MANAGEMENT.md) | Session management |
| [TWO_FACTOR_AUTHENTICATION.md](TWO_FACTOR_AUTHENTICATION.md) | 2FA implementation |
| [IP_WHITELIST.md](IP_WHITELIST.md) | IP whitelist |
| [API_REQUEST_SIGNING.md](API_REQUEST_SIGNING.md) | Request signing |

## Contracts & on-chain

| Document | What it covers |
| --- | --- |
| [GAS_COSTS.md](GAS_COSTS.md) | Soroban gas cost reference |
| [contract-event-backfill.md](contract-event-backfill.md) | Contract event backfill |

---

## Archived docs

Docs for features outside the off-ramp product scope (governance, quests, issue-generation tooling, earlier proposals, backlog specs) have been moved to [`docs/archive/`](archive/). They are kept for historical reference only.
