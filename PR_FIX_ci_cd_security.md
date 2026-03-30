# Pull Request: Fix CI/CD and Security Check Failures

## Description
This pull request resolves persistent failures in the CI/CD pipeline, including Clippy, CodeQL, Formatting (fmt), and NPM Security Audits. These issues were identified after the implementation of the request timeout and distributed tracing features.

### Changes:
- **Backend Formatting**: Consolidated `tower_http` imports and standardized the import grouping (std, external, internal) in `backend/src/main.rs` to satisfy `cargo fmt`.
- **Security Upgrade (Hashing)**:
    - Upgraded audit log hashing in `backend/src/admin_audit_log.rs` from insecure **MD5** to **SHA-256**.
    - Replaced manual hex formatting with the idiomatic `hex::encode` function to resolve Clippy lints and improve code quality.
- **Dependency Cleanup**:
    - Removed the insecure `md5` crate from `backend/Cargo.toml`.
    - Removed the redundant `dotenv` crate in favor of `dotenvy`.
    - Resolved a duplicate `redis` entry.
- **Frontend Security**:
    - Updated `dompurify` to version `^3.2.4` to address known vulnerabilities.
    - Stabilized the `next` version to `^15.0.0` to avoid experimental version issues.

## Verification
- Verified code formatting against standard Rust patterns.
- Ensured HMAC/Hashing logic follows cryptographic best practices by using SHA-256.
- Confirmed that redundant/insecure dependencies have been removed.

## Related Issues
#816
