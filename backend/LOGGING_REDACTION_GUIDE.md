# Logging Redaction Guide

## Overview

This guide documents the logging redaction system implemented to ensure GDPR and PCI-DSS compliance by preventing sensitive information from being logged in plaintext.

## Compliance Requirements

### GDPR (General Data Protection Regulation)

- Personal data must be protected and not logged without proper safeguards
- User IDs, email addresses, and IP addresses are considered personal data
- Logs must not contain identifiable information that could be used to track individuals

### PCI-DSS (Payment Card Industry Data Security Standard)

- Transaction data including amounts, account numbers, and payment details must be protected
- Logs must not contain full account numbers or sensitive authentication data
- Audit trails must be maintained without exposing sensitive information

## Redaction Functions

The redaction module (`backend/src/logging/redaction.rs`) provides the following functions:

### 1. `Redacted<T>` Wrapper Type

Completely redacts any value when logged:

```rust
use crate::logging::redaction::Redacted;

let api_key = "sk_live_1234567890abcdef";
tracing::info!("API key: {:?}", Redacted(&api_key));
// Logs: API key: [REDACTED]
```

### 2. `redact_account(account: &str) -> String`

Redacts Stellar account addresses, showing only first 4 and last 4 characters:

```rust
use crate::logging::redaction::redact_account;

let account = "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
tracing::info!(
    account = redact_account(&account),
    "Processing transaction"
);
// Logs: account="GXXX...XXXX"
```

### 3. `redact_amount(amount: f64) -> String`

Redacts payment amounts, showing only order of magnitude:

```rust
use crate::logging::redaction::redact_amount;

let amount = 1234.56;
tracing::info!(
    amount = redact_amount(amount),
    "Payment processed"
);
// Logs: amount="~10^3"
```

### 4. `redact_hash(hash: &str) -> String`

Redacts transaction hashes, showing only first 4 and last 4 characters:

```rust
use crate::logging::redaction::redact_hash;

let tx_hash = "abcdef1234567890abcdef1234567890";
tracing::info!(
    transaction = redact_hash(&tx_hash),
    "Transaction confirmed"
);
// Logs: transaction="abcd...7890"
```

### 5. `redact_user_id(user_id: &str) -> String`

Redacts user IDs, showing only prefix:

```rust
use crate::logging::redaction::redact_user_id;

let user_id = "user_12345678";
tracing::info!(
    user_id = redact_user_id(&user_id),
    "User action logged"
);
// Logs: user_id="user_****"
```

### 6. `redact_email(email: &str) -> String`

Redacts email addresses, showing only domain:

```rust
use crate::logging::redaction::redact_email;

let email = "user@example.com";
tracing::info!(
    email = redact_email(&email),
    "Email sent"
);
// Logs: email="****@example.com"
```

### 7. `redact_ip(ip: &str) -> String`

Redacts IP addresses, showing only first two octets:

```rust
use crate::logging::redaction::redact_ip;

let ip = "192.168.1.100";
tracing::info!(
    client_ip = redact_ip(&ip),
    "Request received"
);
// Logs: client_ip="192.168.*.*"
```

### 8. `redact_token(token: &str) -> String`

Redacts API keys and tokens, showing only first 4 characters:

```rust
use crate::logging::redaction::redact_token;

let token = "sk_live_1234567890abcdef";
tracing::info!(
    token = redact_token(&token),
    "Token validated"
);
// Logs: token="sk_l****"
```

## Usage Patterns

### Structured Logging with Redaction

Always use structured logging with explicit field names and redaction:

```rust
// ✅ GOOD: Structured logging with redaction
tracing::info!(
    user_id = redact_user_id(&user.id),
    account = redact_account(&stellar_account),
    amount = redact_amount(payment.amount),
    "Payment processed successfully"
);

// ❌ BAD: String interpolation without redaction
tracing::info!("Payment processed for user {} with amount {}", user.id, payment.amount);

// ❌ BAD: Debug formatting without redaction
tracing::info!("Payment details: {:?}", payment);
```

### Error Logging

When logging errors, avoid including sensitive data in error messages:

```rust
// ✅ GOOD: Error logging without sensitive data
tracing::error!(
    error = %e,
    user_id = redact_user_id(&user_id),
    "Failed to process payment"
);

// ❌ BAD: Error with sensitive data
tracing::error!("Failed to process payment for user {}: {:?}", user_id, payment);
```

### Debug Logging

Debug logs should also use redaction:

```rust
// ✅ GOOD: Debug with redaction
tracing::debug!(
    user_id = redact_user_id(&user_id),
    token_type = "refresh",
    "Token stored in Redis"
);

// ❌ BAD: Debug without redaction
tracing::debug!("Stored refresh token for user: {}", user_id);
```

## Testing Redaction

Run the redaction tests:

```bash
cd backend
cargo test --lib logging::redaction::tests
```

## Checking for Sensitive Data in Logs

### Manual Inspection

Run the application and check logs for patterns:

```bash
# Check for Stellar addresses (56 character strings starting with G)
cargo run 2>&1 | grep -E "G[A-Z0-9]{55}"

# Check for potential user IDs (numeric patterns)
cargo run 2>&1 | grep -E "[0-9]{8,}"

# Check for email addresses
cargo run 2>&1 | grep -E "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"

# Check for IP addresses
cargo run 2>&1 | grep -E "\b([0-9]{1,3}\.){3}[0-9]{1,3}\b"
```

### Automated Scanning

Use the provided script to scan for sensitive patterns:

```bash
cd backend
./scripts/check_sensitive_logs.sh
```

## Migration Checklist

When adding new logging statements, ensure:

- [ ] No user IDs are logged in plaintext
- [ ] No Stellar account addresses are logged in full
- [ ] No payment amounts are logged in plaintext
- [ ] No transaction hashes are logged in full
- [ ] No API keys or tokens are logged
- [ ] No email addresses are logged in full
- [ ] No IP addresses are logged in full
- [ ] Structured logging is used with explicit field names
- [ ] Appropriate redaction functions are applied
- [ ] Debug formatting (`{:?}`) is avoided for sensitive types

## Common Patterns to Avoid

### 1. String Interpolation

```rust
// ❌ AVOID
tracing::info!("User {} performed action", user_id);

// ✅ USE
tracing::info!(
    user_id = redact_user_id(&user_id),
    "User performed action"
);
```

### 2. Debug Formatting

```rust
// ❌ AVOID
tracing::info!("Payment: {:?}", payment);

// ✅ USE
tracing::info!(
    payment_id = redact_hash(&payment.id),
    amount = redact_amount(payment.amount),
    "Payment processed"
);
```

### 3. Logging Entire Structs

```rust
// ❌ AVOID
tracing::debug!("Request: {:?}", request);

// ✅ USE
tracing::debug!(
    method = %request.method,
    path = %request.path,
    "Request received"
);
```

## Environment Variables

Control logging behavior with these environment variables:

- `RUST_LOG`: Set log level (e.g., `info`, `debug`, `trace`)
- `LOG_FORMAT`: Set to `json` for structured JSON logs
- `LOG_DIR`: Directory for log file output (optional)
- `OTEL_ENABLED`: Enable OpenTelemetry tracing (`true`/`false`)

## Audit and Compliance

### Log Retention

Logs are automatically rotated daily with a maximum of 30 files retained (configurable in `observability/tracing.rs`).

### Access Control

Ensure log files have appropriate permissions:

```bash
chmod 640 /path/to/logs/*.log
chown app:app /path/to/logs/*.log
```

### Regular Audits

Schedule regular audits to check for sensitive data leakage:

1. Review new logging statements in code reviews
2. Run automated scans on production logs
3. Monitor for compliance violations
4. Update redaction rules as needed

## References

- [GDPR Article 32: Security of Processing](https://gdpr-info.eu/art-32-gdpr/)
- [PCI-DSS Requirement 3: Protect Stored Cardholder Data](https://www.pcisecuritystandards.org/)
- [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)

## Support

For questions or issues with the logging redaction system, contact the security team or open an issue in the repository.
