#!/usr/bin/env node
/**
 * Generate the SEP-10 interop fixture for issue #2545.
 *
 * Builds a SEP-10 challenge transaction *and signs it* exactly the way a
 * JavaScript wallet/integration would, using @stellar/stellar-sdk, then
 * writes it to `backend/tests/sep/fixtures/sep10_signed_challenge.json`.
 * The Rust test `interop_stellar_sdk_signed_challenge_verifies` in
 * `backend/src/auth/sep10_simple.rs` reads that fixture and must accept it:
 * proof that the Rust challenge/verification logic and the ecosystem's
 * reference SDK agree byte-for-byte.
 *
 * Run from the repository:
 *   cd frontend && node scripts/generate-sep10-fixture.mjs
 *
 * The keys are deterministic TEST keys (raw seeds [11;32] and [7;32]) that
 * mirror the Rust unit tests — never use them for anything real.
 *
 * Time bounds in the fixture are deliberately long-lived (min 0, max 2100)
 * so the committed fixture never expires; production challenges use the
 * SEP-10-recommended 15-minute window, which the Rust unit tests cover.
 */
import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { randomBytes } from 'node:crypto';
import {
  Account,
  Keypair,
  Operation,
  TransactionBuilder,
} from '@stellar/stellar-sdk';

const NETWORK_PASSPHRASE = 'Test SDF Network ; September 2015';
const HOME_DOMAIN = 'example.com';
const WEB_AUTH_ENDPOINT = `https://${HOME_DOMAIN}/api/sep10/auth`;

const server = Keypair.fromRawEd25519Seed(Buffer.alloc(32, 11));
const client = Keypair.fromRawEd25519Seed(Buffer.alloc(32, 7));

// SEP-10: 48 random bytes, base64-encoded into the 64-byte ManageData value.
const nonce = randomBytes(48).toString('base64');

const MIN_TIME = 0; // 1970-01-01 — fixture must stay valid forever
const MAX_TIME = 4102444800; // 2100-01-01

const challenge = new TransactionBuilder(
  // Sequence 0: the challenge must be impossible to submit on-chain.
  // Sequence 0: the challenge must be impossible to submit on-chain, and the
  // Rust verifier requires it. TransactionBuilder bumps the source sequence by
  // one (source N -> tx N+1), so the account's sequence is seeded at -1 to
  // land on 0. This is a fixture-only trick; the account never exists on-chain.
  new Account(server.publicKey(), '-1'),
  { fee: '100', networkPassphrase: NETWORK_PASSPHRASE },
)
  .addOperation(
    // Operation 1: nonce, sourced by the client account.
    Operation.manageData({
      source: client.publicKey(),
      name: `${HOME_DOMAIN} auth`,
      value: nonce,
    }),
  )
  .addOperation(
    // Operation 2: web_auth_domain, sourced by the server (transaction
    // source by default).
    Operation.manageData({
      name: 'web_auth_domain',
      value: new URL(WEB_AUTH_ENDPOINT).host,
    }),
  )
  .setTimebounds(MIN_TIME, MAX_TIME)
  .build();

// Server signs first (the challenge signature), then the client.
challenge.sign(server);
challenge.sign(client);

const fixture = {
  description:
    'SEP-10 challenge built and signed with @stellar/stellar-sdk; must verify in Rust. Regenerate with: cd frontend && node scripts/generate-sep10-fixture.mjs',
  network_passphrase: NETWORK_PASSPHRASE,
  home_domain: HOME_DOMAIN,
  web_auth_endpoint: WEB_AUTH_ENDPOINT,
  server_public_key: server.publicKey(),
  server_seed: server.secret(),
  client_account: client.publicKey(),
  transaction: challenge.toEnvelope().toXDR('base64'),
};

const here = dirname(fileURLToPath(import.meta.url));
const outPath = resolve(
  here,
  '../../backend/tests/sep/fixtures/sep10_signed_challenge.json',
);
mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, `${JSON.stringify(fixture, null, 2)}\n`);
console.log(`wrote ${outPath}`);
