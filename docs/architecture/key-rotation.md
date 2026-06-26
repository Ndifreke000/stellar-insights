# Cryptographic Key Rotation Architecture

This document describes how to safely rotate three types of cryptographic keys used in Stellar Insights without service disruption: JWT_SECRET, ENCRYPTION_KEY, and SEP-10 server keypair.

## Overview

| Key Type | Purpose | Rotation Interval | Downtime Required |
|-----------|---------|-------------------|-------------------|
| JWT_SECRET | Sign and verify session tokens | 90 days | 0 (dual-validation window) |
| ENCRYPTION_KEY | Encrypt sensitive data at rest | 1 year | Yes (data migration) |
| SEP-10 keypair | Anchor authentication on Stellar network | 1 year or on compromise | No (overlap period) |

---

## 1. JWT_SECRET Rotation (Zero-Downtime)

### Architecture

JWT tokens are short-lived (default: 1 hour) and used for API authentication. Rotation uses a **dual-validation window** to ensure no active sessions are invalidated.

### Procedure

#### Phase 1: Preparation (Before Rotation)
1. Identify current JWT_SECRET from your deployment:
   ```bash
   # In production environment
   echo $JWT_SECRET
   ```

2. Save the old secret for reference (you'll use it to set JWT_SECRET_OLD).

#### Phase 2: Rotation (Deployment Step)
1. Generate new JWT_SECRET:
   ```bash
   openssl rand -base64 48
   ```

2. Update your deployment configuration (e.g., `.env.production`, AWS Secrets Manager, Kubernetes secrets):
   ```bash
   # For dual-validation window setup
   JWT_SECRET_OLD=<old_secret_from_phase_1>   # Accept old tokens for 15 minutes
   JWT_SECRET=<new_secret_from_step_1>        # Sign new tokens with this
   ```

3. Restart the backend service:
   ```bash
   # Kubernetes example
   kubectl rollout restart deployment/stellar-insights-backend
   
   # Or docker-compose
   docker-compose restart backend
   ```

4. Verify backend logs show validation against both secrets:
   ```bash
   docker logs stellar-insights-backend | grep -i "jwt\|token\|validation"
   # Expected: No authentication errors for 15 minutes
   ```

#### Phase 3: Finalization (After 15 Minutes)
1. Once the 15-minute overlap window expires, remove the old secret:
   ```bash
   # Remove JWT_SECRET_OLD from your configuration
   unset JWT_SECRET_OLD
   # Or delete it from Secrets Manager / Kubernetes Secrets
   ```

2. Restart backend again to complete rotation:
   ```bash
   kubectl rollout restart deployment/stellar-insights-backend
   ```

3. Verify only new secret is in use:
   ```bash
   # Check logs for no JWT validation errors
   docker logs stellar-insights-backend | tail -50 | grep -i "jwt"
   ```

### Validation

- Existing sessions remain valid throughout the rotation
- New sessions use the new JWT_SECRET
- Client applications do not need to re-authenticate
- No API requests fail due to token validation

### Rollback

If issues occur during the overlap window:
1. Keep JWT_SECRET_OLD in place
2. Monitor error rates (should remain < 0.1%)
3. Once stable, proceed to Phase 3

If issues occur after removing JWT_SECRET_OLD:
1. Restore JWT_SECRET_OLD immediately
2. Restart backend
3. Investigate token validation errors in logs
4. Do not retry rotation until root cause is identified

---

## 2. ENCRYPTION_KEY Rotation (Requires Downtime)

### Architecture

ENCRYPTION_KEY encrypts sensitive data at rest (e.g., API keys, PII, private keys). Unlike JWT_SECRET, rotation requires **re-encrypting all encrypted fields** in the database with the new key.

### Procedure

#### Phase 1: Planning
1. Identify all encrypted fields in the database:
   ```sql
   -- Find encrypted columns (typically stored as VARBINARY or TEXT)
   SELECT column_name, data_type 
   FROM information_schema.columns 
   WHERE table_schema = 'stellar_insights'
     AND column_name LIKE '%encrypted%' OR column_name LIKE '%secret%';
   ```

2. Backup the entire database:
   ```bash
   # PostgreSQL
   pg_dump stellar_insights > backup_$(date +%Y%m%d_%H%M%S).sql
   
   # SQLite
   cp stellar_insights.db stellar_insights.db.backup_$(date +%Y%m%d_%H%M%S)
   ```

3. Schedule a maintenance window (all API requests will fail during this time):
   - Estimate: 1 minute per 100MB of encrypted data
   - Notify users 24 hours before
   - Set up status page message

#### Phase 2: Execute Migration
1. Stop the backend service:
   ```bash
   docker-compose down backend
   ```

2. Stop any scheduled jobs that might write data:
   ```bash
   # Disable cron jobs, ingestion processes, webhooks
   ```

3. Generate new ENCRYPTION_KEY:
   ```bash
   openssl rand -hex 32
   ```

4. Create migration script that:
   - Connects with OLD ENCRYPTION_KEY
   - Decrypts all fields
   - Reconnects with NEW ENCRYPTION_KEY
   - Re-encrypts all fields
   - Verifies data integrity

   Example migration (Rust):
   ```rust
   use sqlx::sqlite::SqlitePool;
   
   #[tokio::main]
   async fn main() -> anyhow::Result<()> {
       let old_pool = SqlitePool::connect(&format!(
           "sqlite://stellar_insights.db?key={}",
           std::env::var("OLD_ENCRYPTION_KEY")?
       )).await?;
       
       let new_key = std::env::var("NEW_ENCRYPTION_KEY")?;
       
       // Fetch all rows with encrypted fields
       let rows = sqlx::query("SELECT id, encrypted_field FROM table_name")
           .fetch_all(&old_pool).await?;
       
       // Re-encrypt with new key
       for row in rows {
           let id: i64 = row.get("id");
           let decrypted = decrypt(&row.get::<Vec<u8>, _>("encrypted_field"))?;
           let re_encrypted = encrypt(&decrypted, &new_key)?;
           
           sqlx::query("UPDATE table_name SET encrypted_field = ? WHERE id = ?")
               .bind(re_encrypted).bind(id)
               .execute(&old_pool).await?;
       }
       
       Ok(())
   }
   ```

5. Run the migration:
   ```bash
   OLD_ENCRYPTION_KEY=$OLD_KEY \
   NEW_ENCRYPTION_KEY=$NEW_KEY \
   cargo run --bin migrate-encryption-key
   ```

6. Verify re-encryption:
   ```sql
   -- Sample check: decrypt a field with new key
   SELECT COUNT(*) as total_records FROM table_name;
   -- Should match pre-migration count
   ```

#### Phase 3: Deployment
1. Update your deployment with new ENCRYPTION_KEY:
   ```bash
   kubectl set env deployment/stellar-insights-backend \
     ENCRYPTION_KEY=$NEW_KEY
   ```

2. Restart backend service:
   ```bash
   docker-compose up -d backend
   ```

3. Monitor logs for decryption errors:
   ```bash
   docker logs -f stellar-insights-backend | grep -i decrypt
   # Should show no errors
   ```

4. Run smoke tests:
   ```bash
   ./scripts/smoke-test.sh
   ```

### Validation

- All encrypted fields decrypt correctly with new key
- No data loss observed
- Application functionality restored
- Database size unchanged (encryption has similar payload size)

### Rollback

If decryption fails after deploying new key:
1. Restore backup database:
   ```bash
   docker-compose down backend
   cp stellar_insights.db.backup ~/
   docker-compose up -d backend
   ```

2. Revert ENCRYPTION_KEY in deployment:
   ```bash
   kubectl set env deployment/stellar-insights-backend \
     ENCRYPTION_KEY=$OLD_KEY
   ```

3. Investigate root cause before retrying
4. Do NOT retry rotation with corrupted encrypted data

---

## 3. SEP-10 Server Keypair Rotation

### Architecture

The SEP-10 server keypair is a Stellar Ed25519 keypair used for anchor authentication. It must be registered with the Stellar network and can be rotated without service interruption by maintaining a short overlap period.

### Procedure

#### Phase 1: Generate New Keypair
1. Generate new Stellar keypair using one of these methods:

   **Option A: Stellar CLI**
   ```bash
   stellar keys generate
   # Output: Public: GBRPYHIL2CI3C7OHIXBBYLCA3VU2MXXIIVU2KHV25QVKNUKE6FSKIZB
   # Output: Secret: SBTVMQA2D7AIRVZLOF7JJLSOIS2Z5FEFXWXVF5GHMQETOHQSQQQQQQ
   ```

   **Option B: Stellar SDK (Node.js)**
   ```bash
   npm install stellar-sdk
   node -e "const StellarSdk = require('stellar-sdk'); const kp = StellarSdk.Keypair.random(); console.log('Public:', kp.publicKey()); console.log('Secret:', kp.secret());"
   ```

   **Option C: Python**
   ```bash
   pip install stellar-sdk
   python3 -c "from stellar_sdk import Keypair; kp = Keypair.random(); print('Public:', kp.public_key); print('Secret:', kp.secret)"
   ```

2. Save both public and secret keys securely (never commit to git).

#### Phase 2: Register New Public Key
1. Add new public key to Stellar anchor registration:
   ```bash
   # Update your anchor server's TOML file
   # https://your-anchor.example.com/.well-known/stellar.toml
   
   SIGNING_KEY = "GBRPYHIL2CI3C7OHIXBBYLCA3VU2MXXIIVU2KHV25QVKNUKE6FSKIZB"  # Old key
   SIGNING_KEY = "GBGWZ77LQYVRJ6JMVCZAXKNBEMLL7ZHP2BTPHYWQZ6OWBHQQ3J4XSEZ"  # New key (add this)
   ```

2. Wait for Stellar network to propagate the change (typically 5-10 minutes):
   ```bash
   # Verify TOML is accessible
   curl -s https://your-anchor.example.com/.well-known/stellar.toml | grep SIGNING_KEY
   ```

#### Phase 3: Deploy New Secret Key (Overlap Period)
1. Update your backend configuration to accept BOTH keypairs temporarily:
   ```bash
   # In .env.production or secrets manager
   SEP10_KEYPAIR_OLD="SBTVMQA2D7AIRVZLOF7JJLSOIS2Z5FEFXWXVF5GHMQETOHSQQQQQQQQ"  # Old private key
   SEP10_KEYPAIR="SBGXEKBUZXZDYKDMWYGZDKLKIQGF2NBCYTCFTQV3PZ7BGZWXQVZQXMQ"    # New private key
   ```

2. Restart backend (now validating with both old and new keys):
   ```bash
   kubectl rollout restart deployment/stellar-insights-backend
   ```

3. Monitor SEP-10 authentication (should succeed for both old and new keys):
   ```bash
   # Test SEP-10 challenge
   curl https://api.stellar-insights.com/auth?account=GBRPYHIL...
   # Should return a challenge signed by both the old and new key
   ```

#### Phase 4: Finalize (After 1-2 Hours)
1. Once both keys are validated on the network, remove the old key from your config:
   ```bash
   unset SEP10_KEYPAIR_OLD
   ```

2. Restart backend to use only the new key:
   ```bash
   kubectl rollout restart deployment/stellar-insights-backend
   ```

3. Update the TOML to remove the old key:
   ```bash
   # Update https://your-anchor.example.com/.well-known/stellar.toml
   # Remove the old SIGNING_KEY entry, keep only the new one
   ```

### Validation

- SEP-10 authentication challenges are signed with new keypair
- No increase in authentication failures
- Clients receive valid challenge responses
- Network integration tests pass

### Rollback

If SEP-10 authentication fails:
1. Restore SEP10_KEYPAIR_OLD:
   ```bash
   kubectl set env deployment/stellar-insights-backend \
     SEP10_KEYPAIR_OLD=$OLD_KEY
   ```

2. Keep both keys in TOML until root cause is identified
3. Investigate authentication error logs
4. Do not retry rotation until issues are resolved

---

## Monitoring & Alerting

### Metrics to Monitor During Rotation
- Authentication failure rate (should remain < 0.1%)
- Data decryption errors (should be 0)
- API latency (should not spike)
- Database transaction failures (should be 0 during non-ENCRYPTION_KEY rotations)

### Alerts to Set Up
```yaml
# Example Prometheus alerts
- alert: JWTValidationErrors
  expr: rate(jwt_validation_errors[5m]) > 0.001
  annotations:
    summary: "JWT validation errors during rotation"

- alert: EncryptionKeyErrors
  expr: rate(encryption_key_errors[5m]) > 0
  annotations:
    summary: "Data decryption errors - possible ENCRYPTION_KEY mismatch"

- alert: SEP10AuthErrors
  expr: rate(sep10_auth_errors[5m]) > 0.001
  annotations:
    summary: "SEP-10 authentication errors - possible keypair issue"
```

### Audit Logging

Log all key rotation events:
```json
{
  "timestamp": "2024-01-15T14:30:00Z",
  "event": "jwt_secret_rotated",
  "user": "admin@example.com",
  "status": "success",
  "overlap_window_minutes": 15,
  "verified_at": "2024-01-15T14:45:00Z"
}
```

---

## Quick Reference

### Run Key Rotation Script
```bash
source .env.production
./scripts/rotate-keys.sh
```

### Emergency Key Recovery
If a key is compromised:
1. **JWT_SECRET compromise**: Rotate immediately (15-min overlap window)
2. **ENCRYPTION_KEY compromise**: Rotate immediately (requires downtime)
3. **SEP-10 keypair compromise**: Rotate immediately and revoke old key from TOML

### Troubleshooting

| Symptom | Cause | Resolution |
|---------|-------|-----------|
| JWT validation failures | Old secret not in JWT_SECRET_OLD during overlap | Check env vars, ensure both are set |
| Decryption failures | Wrong ENCRYPTION_KEY deployed | Revert to backup, check key in deployment |
| SEP-10 auth fails | New key not registered in TOML | Update TOML and wait for propagation |
| High latency after rotation | Old key still being used in cache | Clear caches, verify key deployment |

---

## Related Documentation
- [Stellar SEP-10: Stellar Web Authentication](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046.md)
- [NIST Key Rotation Guidelines](https://csrc.nist.gov/publications/detail/sp/800-57/part-1/final)
- Backend Configuration: See `.env.example` for all cryptographic key environment variables
