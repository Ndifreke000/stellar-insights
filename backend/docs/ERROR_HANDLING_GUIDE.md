# Error Handling Guide

## Hierarchy

- `DomainError`: business/domain validation and rule violations.
- `ApiError`: HTTP-facing error envelope used by handlers.
- `anyhow::Result<T>`: internal service/repository fallible operations where flexibility is needed.

## Recommended Usage

- Domain logic and validators should return `Result<T, DomainError>`.
- API handlers should return `ApiResult<T>` and rely on `From` conversions.
- Infrastructure-heavy internals can use `anyhow::Result<T>` and convert at API boundaries.

## Conversion Rules

- `DomainError` -> `ApiError`: via `impl From<DomainError> for ApiError`.
- `sqlx::Error` -> `ApiError`: via existing conversion impl.
- `crate::rpc::error::RpcError` -> `ApiError`: via conversion impl.

## Anti-Patterns To Avoid

- `Result<T, String>` in new domain or API code.
- Manual string mapping at every API boundary.
- Mixing ad-hoc custom response errors when `ApiError` can represent the failure.
