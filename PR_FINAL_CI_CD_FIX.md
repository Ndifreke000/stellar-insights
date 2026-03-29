# Pull Request: Final CI/CD and Security Resolution

## Description
This pull request provides the final resolution for persistent CI/CD and security failures across the backend and frontend. It specifically addresses the broken installation stage in the frontend and the formatting lints in the backend.

### Changes:
- **Frontend Installation Fix**: Reverted `frontend/package.json` to match the existing `package-lock.json`. This ensures that `npm ci` succeeds during the CI/CD pipeline, allowing security scans and CodeQL to execute properly.
- **Backend Formatting & Lints**:
    *   **Resolved Redundant Imports**: Removed duplicate `utoipa` and `utoipa_swagger_ui` imports in `backend/src/main.rs`.
    *   **Standardized `fmt` Check**: Reordered and grouped imports in `main.rs` and `admin_audit_log.rs` to strictly follow `rustfmt` conventions.
    *   **Upgraded Hashing Mechanism**: Transitioned the audit log from MD5 to SHA-256 and implemented the idiomatic `hex::encode` for hex formatting to satisfy security lints.
- **Backend Compilation Resolution**:
    *   **Fixed Duplicate axum / utoipa Imports**: Resolved `State` and `IntoResponse` collisions in `handlers.rs` and `anchors.rs`.
    *   **Stellar SDK Type Transition**: Migrated `contract.rs` to use `stellar-xdr` for core types (Memo, Transaction, Preconditions) as recommended by the compiler.
    *   **Service Pathing**: Corrected incorrect module paths in `contract_listener.rs` and updated `from_env()` to include required `AlertService` initialization.
    *   **Dependency Cleanup**: Removed insecure `md5` and redundant `dotenv` crates from the backend.

## Verification
- Confirmed that `package.json` now matches the dependencies in `package-lock.json`.
- Manually reviewed backend formatting against project `rustfmt.toml` rules.
- Verified that all hashing logic follows modern cryptographic standards (SHA-256).

## Related Issues
#816
