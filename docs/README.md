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
| [design/2856-async-data-export.md](design/2856-async-data-export.md) | Design note #2856: async export jobs, Parquet/CSV, signed resumable downloads |
| [design/2857-trustless-snapshot-verification.md](design/2857-trustless-snapshot-verification.md) | Design note #2857: browser-side snapshot verification against the chain |
| [design/2858-contract-event-listener-cursor.md](design/2858-contract-event-listener-cursor.md) | Design note #2858: contract event cursor expiry, pagination, gap recording |
| [design/2859-admin-rbac.md](design/2859-admin-rbac.md) | Design note #2859: admin roles, permissions matrix, enforcement and audit |

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
| [CORRIDOR_ALERT_RULES_ENGINE.md](CORRIDOR_ALERT_RULES_ENGINE.md) | Composable corridor alert rules and backtesting (#2855) |
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
| [security/MOBILE_SEP10_KEY_STORAGE.md](security/MOBILE_SEP10_KEY_STORAGE.md) | Proposed: mobile SEP-10 key custody, biometric-gated signing, threat model (#2845) |
| [security/SECURITY_TESTING_SCOPE.md](security/SECURITY_TESTING_SCOPE.md) | Security testing scope |
| [SECRETS_MANAGEMENT.md](SECRETS_MANAGEMENT.md) | Secrets management |
| [AUDIT_LOG.md](AUDIT_LOG.md) | Audit logging |
| [SESSION_MANAGEMENT.md](SESSION_MANAGEMENT.md) | Session management |
| [TWO_FACTOR_AUTHENTICATION.md](TWO_FACTOR_AUTHENTICATION.md) | 2FA implementation |
| [2FA_WEBAUTHN_AND_RECOVERY_CODES.md](2FA_WEBAUTHN_AND_RECOVERY_CODES.md) | WebAuthn/passkeys, recovery codes, admin 2FA (#2852) |
| [OAUTH_PKCE_STATE_AND_ACCOUNT_LINKING.md](OAUTH_PKCE_STATE_AND_ACCOUNT_LINKING.md) | OAuth PKCE, state binding, account linking (#2853) |
| [MULTISIG_COORDINATION_SERVICE.md](MULTISIG_COORDINATION_SERVICE.md) | Multisig signature collection and submission (#2854) |
| [IP_WHITELIST.md](IP_WHITELIST.md) | IP whitelist |
| [API_REQUEST_SIGNING.md](API_REQUEST_SIGNING.md) | Request signing |
| [SUPPLY_CHAIN_SECURITY.md](SUPPLY_CHAIN_SECURITY.md) | SBOMs, SLSA provenance, signed images, npm/PyPI provenance (#2848) |

## CI & testing

| Document | What it covers |
| --- | --- |
| [CI_PIPELINE.md](CI_PIPELINE.md) | Workflow inventory and consolidation into reusable, path-filtered pipelines (#2849) |
| [PROPERTY_BASED_TESTING.md](PROPERTY_BASED_TESTING.md) | Aggregation invariants and proptest strategy (#2850) |
| [MUTATION_TESTING.md](MUTATION_TESTING.md) | cargo-mutants on auth, 2FA, crypto and the contract (#2851) |

## Contracts & on-chain

| Document | What it covers |
| --- | --- |
| [GAS_COSTS.md](GAS_COSTS.md) | Soroban gas cost reference |
| [contract-event-backfill.md](contract-event-backfill.md) | Contract event backfill |

---

## Archived docs

Docs for features outside the off-ramp product scope (governance, quests, issue-generation tooling, earlier proposals, backlog specs) have been moved to [`docs/archive/`](archive/). They are kept for historical reference only.
