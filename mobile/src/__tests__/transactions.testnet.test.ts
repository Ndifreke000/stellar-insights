/**
 * Testnet smoke tests for transaction flows.
 * Run with: jest --testPathPattern=testnet
 *
 * These tests verify that the Stellar testnet can accept and process
 * transactions submitted by the mobile app. They require network access
 * and use Friendbot-funded ephemeral keypairs so no real funds are at risk.
 */

import axios from 'axios';
import { Keypair, Networks, TransactionBuilder, BASE_FEE, Operation, Asset } from '@stellar/stellar-sdk';

const TESTNET_HORIZON = 'https://horizon-testnet.stellar.org';
const FRIENDBOT_URL = 'https://friendbot.stellar.org';
const REQUEST_TIMEOUT = 30_000;

const http = axios.create({ timeout: REQUEST_TIMEOUT });

async function fundAccount(publicKey: string): Promise<void> {
  await http.get(FRIENDBOT_URL, { params: { addr: publicKey } });
}

async function loadAccount(publicKey: string) {
  const response = await http.get(`${TESTNET_HORIZON}/accounts/${publicKey}`);
  return response.data;
}

async function submitTransaction(txXdr: string) {
  return http.post(
    `${TESTNET_HORIZON}/transactions`,
    new URLSearchParams({ tx: txXdr }),
    { headers: { 'Content-Type': 'application/x-www-form-urlencoded' } },
  );
}

describe('[testnet] Transaction submission smoke tests', () => {
  let sourceKeypair: Keypair;
  let destinationKeypair: Keypair;

  beforeAll(async () => {
    sourceKeypair = Keypair.random();
    destinationKeypair = Keypair.random();

    // Fund the source account via Friendbot; destination is created by the tx.
    await fundAccount(sourceKeypair.publicKey());
  }, REQUEST_TIMEOUT);

  it('loads funded source account from Horizon', async () => {
    const account = await loadAccount(sourceKeypair.publicKey());
    expect(account).toHaveProperty('account_id', sourceKeypair.publicKey());
    const xlmBalance = account.balances.find((b: { asset_type: string }) => b.asset_type === 'native');
    expect(xlmBalance).toBeDefined();
    expect(parseFloat(xlmBalance.balance)).toBeGreaterThan(0);
  });

  it('submits a create_account transaction successfully', async () => {
    const account = await loadAccount(sourceKeypair.publicKey());

    const tx = new TransactionBuilder(
      { id: account.account_id, sequence: account.sequence, accountId: () => account.account_id, incrementSequenceNumber: () => {} } as any,
      { fee: BASE_FEE, networkPassphrase: Networks.TESTNET },
    )
      .addOperation(
        Operation.createAccount({
          destination: destinationKeypair.publicKey(),
          startingBalance: '10',
        }),
      )
      .setTimeout(30)
      .build();

    tx.sign(sourceKeypair);

    const response = await submitTransaction(tx.toEnvelope().toXDR('base64'));
    expect(response.status).toBe(200);
    expect(response.data).toHaveProperty('successful', true);
    expect(response.data).toHaveProperty('hash');
  }, REQUEST_TIMEOUT);

  it('returns the submitted transaction on the ledger', async () => {
    const txsResponse = await http.get(
      `${TESTNET_HORIZON}/accounts/${sourceKeypair.publicKey()}/transactions?limit=1&order=desc`,
    );
    expect(txsResponse.status).toBe(200);
    const records = txsResponse.data._embedded.records;
    expect(records.length).toBeGreaterThan(0);
    expect(records[0]).toHaveProperty('successful', true);
  });

  it('rejects a transaction with an invalid signature', async () => {
    const account = await loadAccount(sourceKeypair.publicKey());
    const wrongKeypair = Keypair.random();

    const tx = new TransactionBuilder(
      { id: account.account_id, sequence: account.sequence, accountId: () => account.account_id, incrementSequenceNumber: () => {} } as any,
      { fee: BASE_FEE, networkPassphrase: Networks.TESTNET },
    )
      .addOperation(
        Operation.payment({
          destination: destinationKeypair.publicKey(),
          asset: Asset.native(),
          amount: '1',
        }),
      )
      .setTimeout(30)
      .build();

    // Sign with wrong key — submission should be rejected
    tx.sign(wrongKeypair);

    await expect(
      submitTransaction(tx.toEnvelope().toXDR('base64')),
    ).rejects.toMatchObject({ response: { status: 400 } });
  }, REQUEST_TIMEOUT);
});
