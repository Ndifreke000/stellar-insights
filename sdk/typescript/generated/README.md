# Generated Soroban Bindings

This directory contains auto-generated TypeScript bindings for the Stellar Insights Soroban contracts.

**Do not edit files in this directory manually.** They will be overwritten on the next generation run.

## Regenerating

```bash
# From the sdk/typescript directory:
npm run generate -- --contract-id <YOUR_CONTRACT_ID>

# Or directly:
bash scripts/generate-bindings.sh --contract-id <YOUR_CONTRACT_ID>
```

See `scripts/generate-bindings.sh --help` for all options.

## CI

Bindings are regenerated automatically in CI whenever contract source files change.
See `.github/workflows/sdk-bindings.yml`.
