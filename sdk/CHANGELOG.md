# Changelog

All notable changes to Stellar Insights SDKs are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- TypeScript: Support for custom transaction builders
- Python: Async client for improved performance in concurrent operations

### Changed
- TypeScript: Updated @stellar/stellar-sdk to 14.6.0
- Python: Updated stellar-sdk to 10.5.0

### Fixed
- TypeScript: Memory leak in connection pooling
- Python: Incorrect fee calculation for batch operations

---

## [1.0.0] - 2026-02-20

### Added - TypeScript SDK (v14.5.0)
- Full Stellar Horizon API client with type-safe endpoints
- Soroban contract interaction layer with ABI serialization
- Transaction building and signing with support for multiple key types
- Real-time event streaming for payments and contract invocations
- Batch payment processing with configurable retry logic
- Network abstraction supporting both testnet and mainnet
- Comprehensive error handling and validation for all RPC calls
- Support for custom fee strategies and transaction envelopes

### Added - Python SDK (v10.4.0)
- Pure Python Stellar Horizon API client
- Soroban contract interface with automatic type mapping
- Async transaction streaming using websockets
- Pagination helpers for large dataset queries
- Fee estimation utilities for transaction cost planning
- Network configuration helpers with protocol version detection
- DataFrame export for analytics workflows
- SQLAlchemy integration for persistence layers

### Added - Contract Support (Soroban v22.3.0)
- TypeScript: Stable contract invocation with parameter validation
- Python: Contract spec parsing and dynamic interface generation
- Both: Fee pre-calculation for contract initialization and maintenance

### Changed
- TypeScript: Migrated from callback-based to Promise-based API
- Python: Switched to native async/await from threading model

### Deprecated
- TypeScript: Legacy fee calculation methods (will be removed in v2.0.0)
- Python: Callback-based transaction submission (use async API instead)

### Removed
- TypeScript: Support for Stellar SDK v13.x and older
- Python: Python 3.8 support (now requires 3.9+)

### Fixed
- TypeScript: Race conditions in transaction confirmation polling
- Python: Encoding issues with non-ASCII account metadata
- Both: Incorrect handling of timeout errors during network connectivity issues

### Security
- TypeScript: Audited and patched dependency vulnerabilities (npm audit)
- Python: Validated all external dependency versions against security advisories

---

## [0.9.0] - 2025-11-15

### Added - TypeScript SDK (v14.3.0)
- Experimental Soroban support
- Payment stream monitoring

### Added - Python SDK (v10.2.0)
- Basic Soroban contract support
- Testnet-only features

### Fixed
- Various memory management issues

---

## [0.8.0] - 2025-09-10

### Added - TypeScript SDK (v14.1.0)
- Initial Stellar Horizon API bindings
- Transaction builder utilities

### Added - Python SDK (v10.0.0)
- Initial release with core Horizon API
- Basic payment and account management

### Notes
- Testnet only
- Pre-mainnet release
