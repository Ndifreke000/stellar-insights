# Backup System Documentation

This document describes how the PayRaider SQLite database is protected against
data loss: persistent storage, continuous replication, and periodic local
snapshots. It replaces an earlier version of this document that described an
RDS PostgreSQL backup system; that infrastructure was never actually used by
the application (see `docs/adr/0001-sqlite-vs-postgres.md`) and has since been
removed from Terraform entirely.

## Overview

Three independent layers protect the database, from "survives a restart" to
"survives losing the volume":

| Layer | What it protects against | Mechanism |
|---|---|---|
| **Persistent volume** | Pod/task restarts, redeploys | EFS (ECS) / PVC (k8s), mounted at `/data` |
| **Continuous replication** | Volume loss, corruption, point-in-time recovery | Litestream sidecar → S3 |
| **Periodic local snapshots** | Fast local rollback without touching S3 | `backup.rs`, scheduled daily |

None of these existed as a coherent story before this work — the database
lived on ephemeral Fargate/pod storage with no volume at all, and the only
"backup" configuration in the repo described an RDS instance the backend
can't connect to.

## Layer 1: Persistent Volume

- **ECS**: `aws_efs_file_system.backend_data` (`terraform/modules/compute/ecs/main.tf`),
  mounted via an access point restricted to the container's non-root user
  (uid/gid 1000).
- **k8s**: `k8s/backend/pvc.yaml` (`payraider-backend-data`, 5Gi,
  `ReadWriteOnce`).

Both mount at `/data`; `DATABASE_URL` is `sqlite:///data/payraider.db` in
every environment. The backend is pinned to exactly one replica everywhere
(`desired_count`/`replicas` = 1) because SQLite permits exactly one writer and
there is no shared storage for multiple replicas to safely use the same file.
See ADR 0001 for the full reasoning.

This layer alone does **not** protect against deleting the volume itself, EFS
mount target/AZ failure, or filesystem corruption — that's what Litestream is
for.

## Layer 2: Continuous Replication (Litestream)

A sidecar container (`litestream` in both the ECS task definition and the k8s
Deployment) runs `litestream replicate /data/payraider.db s3://<bucket>/<env>/payraider.db`
continuously, streaming WAL changes to S3 as they happen — not a periodic
job. This is what actually satisfies ADR 0001's durability requirement: a
raw file copy while SQLite is under WAL is not guaranteed consistent, but
Litestream is specifically designed to produce point-in-time-recoverable
replicas from a live WAL-mode database.

- **Bucket**: `terraform/global/backups.tf` — one bucket shared across
  environments, partitioned by key prefix (`s3://payraider-db-backups-<account>/<environment>/`).
  Versioned, SSE-encrypted, public access blocked.
- **Credentials**: the ECS task role, scoped to that environment's prefix only
  (`aws_iam_role_policy.task_litestream_access`). On k8s, either IRSA (preferred,
  if running on EKS) or static keys in the `payraider-secrets` Secret — see
  `k8s/config/secret-template.yaml`.

### Restoring from Litestream

```bash
# Restore the latest replica to a local file
litestream restore -o ./restored.db s3://payraider-db-backups-<account>/<environment>/payraider.db

# Restore to a specific point in time
litestream restore -o ./restored.db -timestamp 2026-09-01T12:00:00Z \
  s3://payraider-db-backups-<account>/<environment>/payraider.db
```

Then copy `restored.db` onto the EFS volume / PVC at `/data/payraider.db`
(with the backend stopped) and restart.

## Layer 3: Periodic Local Snapshots (`backup.rs`)

Independent of Litestream, `backup.rs` runs a daily scheduled job
(`BackupScheduler::spawn_scheduler`, default 2 AM UTC via
`BACKUP_SCHEDULE_HOUR_UTC`) that:

1. Copies the live database file to `BACKUP_DIR` (default `./backups`) as
   `payraider_<timestamp>.db` (`create_backup`).
2. Deletes snapshots older than `BACKUP_RETENTION_DAYS` (default 30)
   (`cleanup_old_backups`).
3. Verifies the resulting file (`verify_backup`).

**Caveat, same one the ADR raises**: this is a raw file copy
(`tokio::fs::copy`), not SQLite's online backup API. Under WAL mode a copy
taken mid-write can be inconsistent. Treat this layer as a fast, convenient
local rollback for the common case (accidental bad migration, obvious data
error caught quickly), not as the durability guarantee — that's Litestream's
job.

**Note**: `BACKUP_DIR` defaults to a path relative to the container's working
directory, which is *not* the `/data` EFS/PVC mount unless explicitly
configured to write there. As shipped, these local snapshots do not survive
a pod/task restart any better than the database itself would without the
volume — set `BACKUP_DIR=/data/backups` if you want them to.

### Manual snapshot / restore

There is currently no API endpoint or CLI command to trigger a snapshot
on demand — `BackupManager::run_once` only runs from the scheduler
(`main.rs` calls `spawn_scheduler` once at startup). To restore: stop the
backend, copy the snapshot over the live file, restart.

```bash
cp ./backups/payraider_20260901_020000.db /data/payraider.db
```

## Monitoring

- Backup size is exported as a Prometheus gauge
  (`crate::observability::metrics::set_backup_size_bytes`).
- Litestream sidecar logs go to the same CloudWatch log group as the backend
  (ECS: `awslogs-stream-prefix = "litestream"`) / the same pod's logs (k8s) —
  watch for replication errors there.
- `write_pool_metrics()` (see `backend/src/database.rs`) surfaces write-pool
  contention, which is a leading indicator of the write volume that would
  eventually stress this whole story (see ADR 0001, "Revisit this decision
  when...").

## Related Documentation

- [ADR 0001: SQLite vs PostgreSQL](adr/0001-sqlite-vs-postgres.md) — why
  there's no RDS, and the hardening this backup system is part of
- [Terraform Infrastructure](../terraform/README.md)
- [Disaster Recovery Plan](disaster-recovery.md)
