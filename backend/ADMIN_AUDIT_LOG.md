# Admin Audit Log System

## Overview
This backend implements a comprehensive, tamper-proof audit log for all admin actions.

### Features
- Logs every admin action (create, update, delete, access, etc.)
- Records: action, resource, user, status, details, timestamp
- Tamper-proof: each log entry includes a hash chained to previous entry
- Storage: `admin_audit_log` table in database
- Easy verification: recompute hash chain to detect tampering

## Schema
See `backend/migrations/015_create_admin_audit_log.sql` for table definition.

## Usage
- Use `AdminAuditLogger` (see `backend/src/admin_audit_log.rs`) to record admin actions:
  ```rust
  db.admin_audit_logger.log_action(
    "delete_user", "users/123", "admin_id", "success", serde_json::json!({}), prev_hash
  ).await?;
  ```
- Integrate in all admin endpoints and flows.

## Tamper-Proof Mechanism
- Each log entry's hash is computed from previous hash and current data.
- To verify, recompute hash chain from first entry.

## Integration
- `AdminAuditLogger` is part of `Database` struct for easy access.
- All admin actions should call `log_action`.

## Example
```rust
let details = serde_json::json!({ "field": "value" });
db.admin_audit_logger.log_action(
  "update_settings", "settings", user_id, "success", details, prev_hash
).await?;
```

## References
- Migration: `backend/migrations/015_create_admin_audit_log.sql`
- Logger: `backend/src/admin_audit_log.rs`
- Database integration: `backend/src/database.rs`
