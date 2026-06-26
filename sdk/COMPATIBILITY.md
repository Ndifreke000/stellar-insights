# SDK Compatibility Matrix

This document outlines the compatibility between Stellar Insights SDK versions and the Stellar ecosystem, including testnet and mainnet support status.

## Compatibility Table

| SDK Version | TypeScript SDK | Python SDK | Contract Version | Testnet Support | Mainnet Support | Notes |
|-------------|----------------|------------|------------------|-----------------|-----------------|-------|
| 1.0.0 | 14.5.0 | 10.4.0 | Soroban v22.3.0 | Yes | Yes | Current stable release |
| 0.9.0 | 14.3.0 | 10.2.0 | Soroban v22.2.0 | Yes | Partial | Legacy version, limited mainnet support |
| 0.8.0 | 14.1.0 | 10.0.0 | Soroban v22.1.0 | Yes | No | Testnet only, pre-mainnet release |

## Versioning Policy

Stellar Insights SDKs follow **Semantic Versioning** (MAJOR.MINOR.PATCH):

- **MAJOR**: Breaking changes (contract ABI changes, removed endpoints, significant API changes)
- **MINOR**: New features, backward-compatible additions (new endpoints, new blockchain data types)
- **PATCH**: Bug fixes and security updates (no API changes)

### What Constitutes a Breaking Change

- Contract ABI modifications (new parameters, removed fields, changed response structure)
- Removed or renamed API endpoints
- Changed response shape or data type for existing fields
- Dropped support for Stellar protocol versions
- Incompatible changes to authentication or authorization patterns

### Checking Compatibility Before Upgrade

1. **Review the changelog** for your current version → target version
2. **Run compatibility check**:
   ```bash
   # TypeScript/JavaScript
   npm audit --audit-level=moderate

   # Python
   pip check
   ```
3. **Check contract compatibility**:
   - Verify contract ABI hash matches expected value
   - Test against Stellar Test Network (testnet) first
4. **Review migration guide** if MAJOR version bump

## Upgrade Safety

### Pre-Upgrade Checklist

1. **Backup configuration and state**:
   ```bash
   # Database
   pg_dump stellar_insights > backup_$(date +%Y%m%d).sql

   # Environment
   cp .env .env.backup
   ```

2. **Test in non-production first**:
   - Deploy to staging environment
   - Run full integration test suite
   - Validate against testnet Stellar nodes

3. **Verify contract compatibility**:
   ```bash
   # Check contract version matches SDK
   soroban contract info --network testnet <CONTRACT_ID>
   ```

4. **Monitor transaction fees**:
   - SDK version may affect fee calculation
   - Test a sample transaction to confirm expected costs

### Upgrade Steps

1. Update SDK version in dependency manifest:
   ```bash
   # TypeScript
   npm install @stellar/stellar-sdk@14.5.0

   # Python
   pip install stellar-sdk==10.4.0
   ```

2. Run test suite:
   ```bash
   npm test  # or pytest
   ```

3. Deploy to staging and monitor for 24 hours

4. Deploy to production with gradual rollout (if using canary deployment)

### Rollback Plan

If issues occur post-upgrade:

1. **Immediate**: Stop new transactions, revert SDK version
2. **Database**: Rollback with backup if schema migrations occurred
3. **Monitoring**: Check transaction/contract call success rates
4. **Communication**: Notify stakeholders of the incident

## Network Support Legend

- **Yes**: Full support for both testnet and mainnet
- **Partial**: Mainnet supported but with known limitations (see Notes)
- **No**: Not supported on this network
