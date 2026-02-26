# Logging Redaction Quick Reference

## Quick Import

```rust
use crate::logging::redaction::{
    redact_account, redact_amount, redact_email,
    redact_hash, redact_ip, redact_token,
    redact_user_id, Redacted
};
```

## Common Patterns

### User IDs

```rust
// ✅ DO THIS
tracing::info!(
    user_id = redact_user_id(&user.id),
    "User action completed"
);

// ❌ NOT THIS
tracing::info!("User {} completed action", user.id);
```

### Stellar Accounts

```rust
// ✅ DO THIS
tracing::info!(
    account = redact_account(&stellar_account),
    "Account processed"
);

// ❌ NOT THIS
tracing::info!("Processing account: {}", stellar_account);
```

### Payment Amounts

```rust
// ✅ DO THIS
tracing::info!(
    amount = redact_amount(payment.amount),
    "Payment processed"
);

// ❌ NOT THIS
tracing::info!("Payment amount: {}", payment.amount);
```

### Transaction Hashes

```rust
// ✅ DO THIS
tracing::warn!(
    tx_hash = redact_hash(&transaction.hash),
    "Transaction failed"
);

// ❌ NOT THIS
tracing::warn!("Transaction {} failed", transaction.hash);
```

### API Keys/Tokens

```rust
// ✅ DO THIS
tracing::debug!(
    token = redact_token(&api_key),
    "Token validated"
);

// ❌ NOT THIS
tracing::debug!("Validating token: {}", api_key);
```

### Complete Redaction

```rust
// ✅ For complete redaction
let secret = "very_secret_value";
tracing::debug!("Secret: {:?}", Redacted(&secret));
// Logs: Secret: [REDACTED]
```

## Redaction Functions Reference

| Function           | Input Example             | Output Example     | Use Case           |
| ------------------ | ------------------------- | ------------------ | ------------------ |
| `redact_account()` | `GXXX...XXX` (56 chars)   | `GXXX...XXXX`      | Stellar addresses  |
| `redact_amount()`  | `1234.56`                 | `~10^3`            | Payment amounts    |
| `redact_hash()`    | `abcd...7890` (32+ chars) | `abcd...7890`      | Transaction hashes |
| `redact_user_id()` | `user_12345678`           | `user_****`        | User identifiers   |
| `redact_email()`   | `user@example.com`        | `****@example.com` | Email addresses    |
| `redact_ip()`      | `192.168.1.100`           | `192.168.*.*`      | IP addresses       |
| `redact_token()`   | `sk_live_abc123...`       | `sk_l****`         | API keys/tokens    |
| `Redacted<T>`      | Any value                 | `[REDACTED]`       | Complete redaction |

## Testing Your Changes

```bash
# Run redaction tests
cargo test --lib logging::redaction

# Check for sensitive data in logs
./scripts/check_sensitive_logs.sh

# Or on Windows
.\scripts\check_sensitive_logs.ps1
```

## Code Review Checklist

Before submitting a PR with logging changes:

- [ ] No user IDs in plaintext
- [ ] No account addresses in full
- [ ] No payment amounts in plaintext
- [ ] No transaction hashes in full
- [ ] No API keys or tokens
- [ ] No email addresses in full
- [ ] No IP addresses in full
- [ ] Using structured logging (not string interpolation)
- [ ] Appropriate redaction functions applied
- [ ] Tests pass
- [ ] Sensitive data scanner passes

## Common Mistakes to Avoid

### 1. String Interpolation

```rust
// ❌ WRONG
tracing::info!("User {} did action", user_id);

// ✅ CORRECT
tracing::info!(
    user_id = redact_user_id(&user_id),
    "User did action"
);
```

### 2. Debug Formatting

```rust
// ❌ WRONG
tracing::info!("Payment: {:?}", payment);

// ✅ CORRECT
tracing::info!(
    payment_id = redact_hash(&payment.id),
    amount = redact_amount(payment.amount),
    "Payment processed"
);
```

### 3. Logging Entire Structs

```rust
// ❌ WRONG
tracing::debug!("Request: {:?}", request);

// ✅ CORRECT
tracing::debug!(
    method = %request.method,
    path = %request.path,
    "Request received"
);
```

## Environment Variables

```bash
# Set log level
export RUST_LOG=info

# Enable debug logging
export RUST_LOG=debug

# JSON format
export LOG_FORMAT=json

# Enable OpenTelemetry
export OTEL_ENABLED=true
```

## Need Help?

- Full guide: `backend/LOGGING_REDACTION_GUIDE.md`
- Implementation details: `backend/LOGGING_REDACTION_IMPLEMENTATION.md`
- Source code: `backend/src/logging/redaction.rs`
