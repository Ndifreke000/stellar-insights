# Logging Redaction Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Application Code                             │
│  (auth.rs, alert_manager.rs, corridors_cached.rs, etc.)        │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ Uses redaction functions
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Logging Module (logging.rs)                     │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │         Redaction Module (logging/redaction.rs)           │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │  • redact_account()   • redact_email()             │  │  │
│  │  │  • redact_amount()    • redact_ip()                │  │  │
│  │  │  • redact_hash()      • redact_token()             │  │  │
│  │  │  • redact_user_id()   • Redacted<T>                │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ Structured logs with redacted fields
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│              Observability Layer (observability/tracing.rs)      │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  • tracing_subscriber (JSON/Text formatting)              │  │
│  │  • OpenTelemetry integration (optional)                   │  │
│  │  • File rotation (daily, 30 files max)                    │  │
│  └───────────────────────────────────────────────────────────┘  │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ Safe, redacted logs
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Log Outputs                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   Console    │  │  Log Files   │  │  OpenTelemetry       │  │
│  │   (stdout)   │  │  (rotating)  │  │  (optional)          │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

### Before Redaction (Non-Compliant)

```
User Action
    │
    ▼
Application Code
    │
    │ tracing::info!("User {} logged in", user_id)
    │ ❌ Sensitive data in plaintext
    ▼
Logs: "User user_12345678 logged in"
    │
    │ ❌ GDPR violation
    │ ❌ PCI-DSS violation
    ▼
Security Risk
```

### After Redaction (Compliant)

```
User Action
    │
    ▼
Application Code
    │
    │ tracing::info!(
    │     user_id = redact_user_id(&user_id),
    │     "User logged in"
    │ )
    ▼
Redaction Function
    │
    │ "user_12345678" → "user_****"
    ▼
Logs: {"user_id":"user_****","message":"User logged in"}
    │
    │ ✅ GDPR compliant
    │ ✅ PCI-DSS compliant
    ▼
Secure Audit Trail
```

## Component Architecture

### 1. Redaction Module

```
┌─────────────────────────────────────────────────────────────┐
│              logging/redaction.rs                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Redacted<T> Wrapper                                        │
│  ├─ impl Debug  → "[REDACTED]"                             │
│  ├─ impl Display → "[REDACTED]"                            │
│  └─ impl Serialize → "[REDACTED]"                          │
│                                                              │
│  Redaction Functions                                        │
│  ├─ redact_account(str) → "GXXX...XXXX"                    │
│  ├─ redact_amount(f64) → "~10^3"                           │
│  ├─ redact_hash(str) → "abcd...7890"                       │
│  ├─ redact_user_id(str) → "user_****"                      │
│  ├─ redact_email(str) → "****@example.com"                 │
│  ├─ redact_ip(str) → "192.168.*.*"                         │
│  └─ redact_token(str) → "sk_l****"                         │
│                                                              │
│  Unit Tests (11 tests)                                      │
│  └─ Comprehensive coverage of all functions                 │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 2. Integration Points

```
┌──────────────────────────────────────────────────────────────┐
│                    Application Modules                        │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  auth.rs                                                     │
│  ├─ Token storage: redact_user_id()                         │
│  └─ Token invalidation: redact_user_id()                    │
│                                                               │
│  services/alert_manager.rs                                   │
│  ├─ Email alerts: redact_user_id()                          │
│  └─ Webhook alerts: redact_user_id()                        │
│                                                               │
│  cache_invalidation.rs                                       │
│  ├─ Anchor cache: redact_user_id()                          │
│  └─ Account cache: redact_account()                         │
│                                                               │
│  api/corridors_cached.rs                                     │
│  ├─ Payment errors: structured logging                      │
│  ├─ Trade warnings: structured logging                      │
│  └─ Payment warnings: redact_hash()                         │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### 3. Testing & Verification

```
┌──────────────────────────────────────────────────────────────┐
│                  Testing Infrastructure                       │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  Unit Tests                                                  │
│  └─ cargo test --lib logging::redaction::tests              │
│                                                               │
│  Sensitive Data Scanner                                      │
│  ├─ check_sensitive_logs.sh (Bash)                          │
│  ├─ check_sensitive_logs.ps1 (PowerShell)                   │
│  └─ Checks 9 sensitive patterns                             │
│                                                               │
│  CI/CD Integration                                           │
│  └─ .github/workflows/logging-compliance-check.yml          │
│     ├─ Runs on every PR                                     │
│     ├─ Unit tests                                           │
│     ├─ Pattern scanning                                     │
│     └─ Clippy checks                                        │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

## Redaction Strategies

### 1. Partial Masking

Used for identifiers where some context is useful:

```
Input:  GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
Output: GXXX...XXXX

Input:  user_12345678
Output: user_****

Input:  192.168.1.100
Output: 192.168.*.*
```

### 2. Order of Magnitude

Used for amounts where scale matters:

```
Input:  1234.56
Output: ~10^3

Input:  50.00
Output: ~10^1

Input:  0.5
Output: ~10^-1
```

### 3. Complete Redaction

Used for highly sensitive data:

```
Input:  sk_live_1234567890abcdef
Output: [REDACTED]

Input:  password123
Output: [REDACTED]
```

## Compliance Mapping

```
┌─────────────────────────────────────────────────────────────┐
│                    GDPR Article 32                           │
│              Security of Processing                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Requirement: Protect personal data                         │
│  ├─ User IDs → redact_user_id()                            │
│  ├─ Email addresses → redact_email()                       │
│  └─ IP addresses → redact_ip()                             │
│                                                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                  PCI-DSS Requirement 3                       │
│           Protect Stored Cardholder Data                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Requirement: Protect transaction data                      │
│  ├─ Payment amounts → redact_amount()                      │
│  ├─ Transaction hashes → redact_hash()                     │
│  └─ Account numbers → redact_account()                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                OWASP Logging Best Practices                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ✅ No sensitive data in logs                               │
│  ✅ Structured logging                                      │
│  ✅ Log injection prevention                                │
│  ✅ Appropriate log levels                                  │
│  ✅ Log retention policies                                  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Performance Characteristics

```
┌─────────────────────────────────────────────────────────────┐
│              Redaction Function Performance                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  redact_account()   : O(1) - String slicing                │
│  redact_amount()    : O(1) - Logarithm calculation         │
│  redact_hash()      : O(1) - String slicing                │
│  redact_user_id()   : O(n) - String search (small n)       │
│  redact_email()     : O(n) - String search (small n)       │
│  redact_ip()        : O(n) - String split (small n)        │
│  redact_token()     : O(1) - String slicing                │
│  Redacted<T>        : O(1) - Constant time                 │
│                                                              │
│  Average overhead: < 1ms per log statement                  │
│  Memory allocation: Minimal (mostly stack)                  │
│  Suitable for: High-frequency logging                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Security Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    Defense in Depth                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Layer 1: Code Level                                        │
│  └─ Redaction functions applied at log site                │
│                                                              │
│  Layer 2: Compile Time                                      │
│  └─ Type system ensures Redacted<T> always redacts         │
│                                                              │
│  Layer 3: CI/CD                                             │
│  └─ Automated scanning for sensitive patterns              │
│                                                              │
│  Layer 4: Runtime                                           │
│  └─ Structured logging prevents injection                  │
│                                                              │
│  Layer 5: Storage                                           │
│  └─ Log rotation limits exposure window                    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Documentation Hierarchy

```
LOGGING_SECURITY_README.md (Start Here)
    │
    ├─ Quick Start Guide
    ├─ Compliance Status
    └─ Tool Overview
        │
        ├─ LOGGING_REDACTION_QUICK_REFERENCE.md
        │   └─ Daily developer reference
        │
        ├─ LOGGING_REDACTION_GUIDE.md
        │   └─ Comprehensive usage guide
        │
        ├─ LOGGING_REDACTION_IMPLEMENTATION.md
        │   └─ Technical architecture
        │
        ├─ LOGGING_REDACTION_ARCHITECTURE.md (This File)
        │   └─ System diagrams
        │
        └─ SENSITIVE_LOGGING_RESOLUTION.md
            └─ Issue resolution details
```

## Future Enhancements

```
┌─────────────────────────────────────────────────────────────┐
│                    Roadmap                                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Phase 1: Core Implementation ✅ COMPLETE                   │
│  ├─ Redaction module                                        │
│  ├─ Update existing logs                                    │
│  └─ Documentation                                           │
│                                                              │
│  Phase 2: Automation ✅ COMPLETE                            │
│  ├─ CI/CD integration                                       │
│  ├─ Sensitive data scanner                                  │
│  └─ Unit tests                                              │
│                                                              │
│  Phase 3: Advanced Features (Planned)                       │
│  ├─ Custom tracing layer for auto-redaction                │
│  ├─ Configuration-based patterns                            │
│  ├─ Encrypted audit logs                                    │
│  └─ Log sanitization tool                                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Conclusion

This architecture provides:

- ✅ **Comprehensive Coverage**: All sensitive data types handled
- ✅ **Type Safety**: Compile-time guarantees via Redacted<T>
- ✅ **Performance**: Minimal overhead (<1ms)
- ✅ **Maintainability**: Clear patterns and documentation
- ✅ **Compliance**: GDPR and PCI-DSS requirements met
- ✅ **Automation**: CI/CD enforcement
- ✅ **Extensibility**: Easy to add new redaction functions

The system is production-ready and provides a solid foundation for secure, compliant logging.
