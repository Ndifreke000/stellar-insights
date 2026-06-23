/**
 * Testnet smoke tests for the mobile API client.
 * Run with: jest --testPathPattern=testnet
 *
 * These tests hit the live Stellar testnet and require network access.
 * They are intentionally excluded from the default jest run.
 */

import axios from 'axios';

const TESTNET_HORIZON = 'https://horizon-testnet.stellar.org';
const TESTNET_RPC = 'https://soroban-testnet.stellar.org';
const REQUEST_TIMEOUT = 15_000;

const http = axios.create({ timeout: REQUEST_TIMEOUT });

describe('[testnet] Horizon API smoke tests', () => {
  it('returns 200 from the root endpoint', async () => {
    const response = await http.get(TESTNET_HORIZON);
    expect(response.status).toBe(200);
  });

  it('returns network_passphrase for testnet', async () => {
    const response = await http.get(TESTNET_HORIZON);
    expect(response.data).toHaveProperty('network_passphrase');
    expect(response.data.network_passphrase).toContain('Test SDF Network');
  });

  it('can fetch account details for a funded testnet account', async () => {
    // Friendbot-funded account used solely for smoke testing
    const testAccount = 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN';
    const response = await http.get(`${TESTNET_HORIZON}/accounts/${testAccount}`);
    expect(response.status).toBe(200);
    expect(response.data).toHaveProperty('account_id', testAccount);
    expect(response.data).toHaveProperty('balances');
    expect(Array.isArray(response.data.balances)).toBe(true);
  });

  it('returns paginated ledger records', async () => {
    const response = await http.get(`${TESTNET_HORIZON}/ledgers?limit=5&order=desc`);
    expect(response.status).toBe(200);
    expect(response.data._embedded.records).toHaveLength(5);
    expect(response.data._embedded.records[0]).toHaveProperty('sequence');
  });

  it('can query fee stats', async () => {
    const response = await http.get(`${TESTNET_HORIZON}/fee_stats`);
    expect(response.status).toBe(200);
    expect(response.data).toHaveProperty('last_ledger');
    expect(response.data).toHaveProperty('fee_charged');
  });
});

describe('[testnet] Soroban RPC smoke tests', () => {
  const rpcPost = (method: string, params: unknown = []) =>
    http.post(TESTNET_RPC, {
      jsonrpc: '2.0',
      id: 1,
      method,
      params,
    });

  it('responds to getHealth', async () => {
    const response = await rpcPost('getHealth');
    expect(response.status).toBe(200);
    expect(response.data.result).toHaveProperty('status', 'healthy');
  });

  it('returns a valid latest ledger', async () => {
    const response = await rpcPost('getLatestLedger');
    expect(response.status).toBe(200);
    const { result } = response.data;
    expect(result).toHaveProperty('sequence');
    expect(typeof result.sequence).toBe('number');
    expect(result.sequence).toBeGreaterThan(0);
  });

  it('returns network passphrase in getNetwork', async () => {
    const response = await rpcPost('getNetwork');
    expect(response.status).toBe(200);
    expect(response.data.result).toHaveProperty('passphrase');
    expect(response.data.result.passphrase).toContain('Test SDF Network');
  });
});
