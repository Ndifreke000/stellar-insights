/**
 * Testnet API smoke tests — run with:
 *   jest --testPathPattern=testnet
 *
 * These tests hit the live Stellar testnet; they require network access
 * and are intentionally excluded from the default CI suite.
 */

// No imports in this file otherwise -- without this, TypeScript treats it as a
// global script rather than a module, and its top-level declarations collide
// with the identically-named ones in transactions.testnet.test.ts.
export {};

const TESTNET_HORIZON = 'https://horizon-testnet.stellar.org';
const TIMEOUT_MS = 30_000;

// Known funded testnet account for read-only checks
const TESTNET_PUBLIC_KEY = 'GAIH3ULLFQ4DGSECF2AR555KZ4KNDGEKN4AFI4SU2M7B43MGK3QJZNWHITE';

async function horizonGet(path: string): Promise<unknown> {
  const res = await fetch(`${TESTNET_HORIZON}${path}`, {
    headers: { Accept: 'application/json' },
  });
  if (!res.ok) {
    throw new Error(`Horizon responded ${res.status} for ${path}`);
  }
  return res.json();
}

describe('Testnet: Horizon connectivity', () => {
  it(
    'returns root metadata from testnet Horizon',
    async () => {
      const data = await horizonGet('/') as Record<string, unknown>;
      expect(typeof data.horizon_version).toBe('string');
      expect(typeof data.core_version).toBe('string');
      expect(data.network_passphrase).toContain('Test SDF Network');
    },
    TIMEOUT_MS,
  );

  it(
    'reports healthy network status on /fee_stats',
    async () => {
      const data = await horizonGet('/fee_stats') as Record<string, unknown>;
      expect(data).toHaveProperty('last_ledger');
      expect(Number(data.last_ledger)).toBeGreaterThan(0);
      expect(data).toHaveProperty('fee_charged');
    },
    TIMEOUT_MS,
  );
});

describe('Testnet: Ledger endpoint', () => {
  it(
    'returns the most recent ledger',
    async () => {
      const data = await horizonGet('/ledgers?order=desc&limit=1') as {
        _embedded: { records: Record<string, unknown>[] };
      };
      const records = data._embedded?.records ?? [];
      expect(records.length).toBeGreaterThan(0);
      const ledger = records[0];
      expect(typeof ledger.sequence).toBe('number');
      expect(Number(ledger.sequence)).toBeGreaterThan(0);
      expect(ledger.network).toBeUndefined(); // Not in ledger response
    },
    TIMEOUT_MS,
  );
});

describe('Testnet: Account endpoint', () => {
  it(
    'returns 404-style problem for a non-existent account',
    async () => {
      // Use an invalid key that won't exist
      const fake = 'GBAD000000000000000000000000000000000000000000000000000BADZZ';
      const res = await fetch(`${TESTNET_HORIZON}/accounts/${fake}`, {
        headers: { Accept: 'application/json' },
      });
      // Horizon returns 400 (invalid key) or 404 (not found)
      expect([400, 404]).toContain(res.status);
    },
    TIMEOUT_MS,
  );
});

describe('Testnet: Assets endpoint', () => {
  it(
    'lists known testnet assets',
    async () => {
      const data = await horizonGet('/assets?limit=5') as {
        _embedded: { records: unknown[] };
      };
      const records = data._embedded?.records ?? [];
      expect(records.length).toBeGreaterThan(0);
    },
    TIMEOUT_MS,
  );
});
