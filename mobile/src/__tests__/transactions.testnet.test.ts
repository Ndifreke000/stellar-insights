/**
 * Testnet transaction smoke tests — run with:
 *   jest --testPathPattern=testnet
 *
 * These tests verify the shape and content of transaction data from the live
 * Stellar testnet. They require network access and are excluded from the default
 * CI suite.
 */

// No imports in this file otherwise -- without this, TypeScript treats it as a
// global script rather than a module, and its top-level declarations collide
// with the identically-named ones in api.testnet.test.ts.
export {};

const TESTNET_HORIZON = 'https://horizon-testnet.stellar.org';
const TIMEOUT_MS = 30_000;

async function horizonGet<T = unknown>(path: string): Promise<T> {
  const res = await fetch(`${TESTNET_HORIZON}${path}`, {
    headers: { Accept: 'application/json' },
  });
  if (!res.ok) {
    throw new Error(`Horizon responded ${res.status} for ${path}`);
  }
  return res.json() as Promise<T>;
}

interface HorizonPage<T> {
  _embedded: { records: T[] };
  _links: {
    self: { href: string };
    next: { href: string };
    prev: { href: string };
  };
}

interface HorizonTransaction {
  id: string;
  hash: string;
  ledger: number;
  created_at: string;
  source_account: string;
  fee_charged: string;
  operation_count: number;
  successful: boolean;
  envelope_xdr: string;
  result_xdr: string;
}

describe('Testnet: Transactions endpoint', () => {
  let latestTxns: HorizonTransaction[];

  beforeAll(async () => {
    const page = await horizonGet<HorizonPage<HorizonTransaction>>(
      '/transactions?order=desc&limit=5&include_failed=false',
    );
    latestTxns = page._embedded?.records ?? [];
  }, TIMEOUT_MS);

  it('returns at least one recent successful transaction', () => {
    expect(latestTxns.length).toBeGreaterThan(0);
  });

  it('transaction records have required fields', () => {
    const tx = latestTxns[0];
    expect(typeof tx.hash).toBe('string');
    expect(tx.hash).toHaveLength(64);
    expect(typeof tx.ledger).toBe('number');
    expect(tx.ledger).toBeGreaterThan(0);
    expect(typeof tx.source_account).toBe('string');
    expect(tx.source_account).toMatch(/^G[A-Z0-9]{55}$/);
    expect(tx.successful).toBe(true);
    expect(typeof tx.operation_count).toBe('number');
    expect(tx.operation_count).toBeGreaterThan(0);
  });

  it('transaction created_at is a valid ISO-8601 timestamp', () => {
    const tx = latestTxns[0];
    const parsed = Date.parse(tx.created_at);
    expect(Number.isNaN(parsed)).toBe(false);
    // Should be a recent date (within the last 30 days)
    const thirtyDaysMs = 30 * 24 * 60 * 60 * 1000;
    expect(Date.now() - parsed).toBeLessThan(thirtyDaysMs);
  });

  it('transaction has non-empty XDR envelope', () => {
    const tx = latestTxns[0];
    expect(typeof tx.envelope_xdr).toBe('string');
    expect(tx.envelope_xdr.length).toBeGreaterThan(0);
  });
});

describe('Testnet: Transaction detail by hash', () => {
  it(
    'fetches a single transaction by hash and verifies its shape',
    async () => {
      const page = await horizonGet<HorizonPage<HorizonTransaction>>(
        '/transactions?order=desc&limit=1&include_failed=false',
      );
      const [first] = page._embedded?.records ?? [];
      if (!first) {
        return; // Testnet can occasionally be quiet
      }

      const tx = await horizonGet<HorizonTransaction>(`/transactions/${first.hash}`);
      expect(tx.hash).toBe(first.hash);
      expect(tx.ledger).toBe(first.ledger);
      expect(tx.successful).toBe(true);
    },
    TIMEOUT_MS,
  );
});

describe('Testnet: Transaction operations', () => {
  it(
    'returns operations for the most recent transaction',
    async () => {
      const page = await horizonGet<HorizonPage<HorizonTransaction>>(
        '/transactions?order=desc&limit=1&include_failed=false',
      );
      const [first] = page._embedded?.records ?? [];
      if (!first) {
        return;
      }

      const ops = await horizonGet<HorizonPage<{ type: string; id: string }>>(
        `/transactions/${first.hash}/operations`,
      );
      const records = ops._embedded?.records ?? [];
      expect(records.length).toBeGreaterThan(0);
      expect(typeof records[0].type).toBe('string');
    },
    TIMEOUT_MS,
  );
});

describe('Testnet: Pagination', () => {
  it(
    'cursor-based pagination returns non-overlapping pages',
    async () => {
      const page1 = await horizonGet<HorizonPage<HorizonTransaction>>(
        '/transactions?order=desc&limit=3&include_failed=false',
      );
      const records1 = page1._embedded?.records ?? [];
      if (records1.length < 3) {
        return; // Not enough transactions to paginate
      }

      const cursor = records1[records1.length - 1].id;
      const page2 = await horizonGet<HorizonPage<HorizonTransaction>>(
        `/transactions?order=desc&limit=3&cursor=${cursor}&include_failed=false`,
      );
      const records2 = page2._embedded?.records ?? [];

      const ids1 = new Set(records1.map(t => t.hash));
      const ids2 = new Set(records2.map(t => t.hash));
      const overlap = [...ids2].filter(id => ids1.has(id));
      expect(overlap).toHaveLength(0);
    },
    TIMEOUT_MS,
  );
});
