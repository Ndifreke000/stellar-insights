/**
 * E2E test: SEP-10 sign-in → create API key → preflight with key.
 *
 * Covers the self-serve funnel end to end against a running backend:
 *   1. POST /api/sep10/auth   — request a challenge for a fresh testnet keypair
 *   2. Sign the challenge with that keypair's secret key
 *   3. POST /api/sep10/verify — exchange the signed challenge for a session token
 *   4. POST /api/api-keys     — create an API key, authenticated with that token
 *   5. GET  /api/v1/preflight — call the public preflight endpoint with the API
 *      key on the Authorization header, and confirm it's recognized (the
 *      response carries API-key-scoped rate-limit headers, not the
 *      anonymous/IP-scoped ones an unauthenticated call would get)
 *
 * This backend's SEP-10 implementation (backend/src/auth/sep10_simple.rs) is
 * a simplified variant: the "transaction" is a base64-encoded JSON blob, not
 * real Stellar XDR, and the required signature is a raw Ed25519 signature
 * (optionally the SEP-53 `"Stellar Signed Message:\n" + message` digest —
 * see `verify_account_signature`) over the UTF-8 bytes of that base64
 * string, checked against the account's StrKey-decoded Ed25519 public key.
 * `Keypair.sign()` from @stellar/stellar-sdk produces exactly that raw
 * signature, so no XDR transaction-building is needed here.
 *
 * The keypair only needs to be a syntactically valid testnet-network
 * account (`G...`, 56 chars) — this flow never checks it against Horizon or
 * requires it to be funded, so it is not funded via Friendbot. Contrast
 * with tests/e2e/testnet/full_payment_flow.rs, which funds its keypair
 * because it exercises a real SEP-24 deposit.
 *
 * Requires a running backend (API_BASE_URL env var, default
 * http://localhost:8080) with SERVER_PORT, DATABASE_URL and REDIS_URL
 * configured — see .github/workflows/e2e-nightly.yml for how CI boots one.
 */
import { test, expect, APIRequestContext } from "@playwright/test";
import { Keypair } from "@stellar/stellar-sdk";

interface ChallengeResponse {
  transaction: string;
  network_passphrase: string;
}

interface VerificationResponse {
  token: string;
  expires_in: number;
}

interface CreateApiKeyResponse {
  key: { id: string; name: string; key_prefix: string };
  plain_key: string;
}

/** Runs the SEP-10 sign-in flow for a fresh keypair and returns the session token. */
async function signInWithSep10(request: APIRequestContext): Promise<string> {
  const keypair = Keypair.random();

  const challengeRes = await request.post("/api/sep10/auth", {
    data: { account: keypair.publicKey() },
  });
  expect(challengeRes.ok(), `challenge request failed: ${await challengeRes.text()}`).toBeTruthy();
  const challenge = (await challengeRes.json()) as ChallengeResponse;
  expect(challenge.transaction).toBeTruthy();

  // Raw Ed25519 signature over the UTF-8 bytes of the challenge string, as
  // `verify_account_signature` expects — see the module doc comment above.
  const signature = keypair.sign(Buffer.from(challenge.transaction, "utf-8")).toString("base64");

  const verifyRes = await request.post("/api/sep10/verify", {
    data: { transaction: challenge.transaction, signature },
  });
  expect(verifyRes.ok(), `verification failed: ${await verifyRes.text()}`).toBeTruthy();
  const verification = (await verifyRes.json()) as VerificationResponse;
  expect(verification.token).toBeTruthy();
  expect(verification.expires_in).toBeGreaterThan(0);

  return verification.token;
}

test.describe("Self-serve funnel: SEP-10 sign-in → API key → preflight", () => {
  test("a freshly created API key authenticates a preflight call", async ({ request }) => {
    const sessionToken = await signInWithSep10(request);

    const createKeyRes = await request.post("/api/api-keys", {
      headers: { Authorization: `Bearer ${sessionToken}` },
      data: { name: `e2e-test-key-${Date.now()}` },
    });
    expect(createKeyRes.status(), await createKeyRes.text()).toBe(201);
    const { plain_key: apiKey } = (await createKeyRes.json()) as CreateApiKeyResponse;
    expect(apiKey).toMatch(/^si_(live|test)_/);

    const preflightRes = await request.get(
      "/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500",
      { headers: { Authorization: `Bearer ${apiKey}` } }
    );
    expect(preflightRes.ok(), await preflightRes.text()).toBeTruthy();

    const preflight = await preflightRes.json();
    expect(["proceed", "caution", "hold", "unknown"]).toContain(preflight.decision);
    expect(typeof preflight.summary).toBe("string");
    expect(Array.isArray(preflight.checks)).toBe(true);

    // The API key was recognized by `api_key_rate_limit_middleware`, which
    // only adds `RateLimit-*` headers once `resolve_api_key_id` finds a
    // matching key for the bearer token — it's a no-op passthrough for
    // anonymous requests (see backend/src/rate_limit.rs and the second
    // test below). This is the signal that the whole funnel actually
    // worked end to end, not just that each step 200'd in isolation.
    expect(preflightRes.headers()["ratelimit-limit"]).toBeTruthy();
  });

  test("an unsigned preflight call still works, but anonymously", async ({ request }) => {
    // Sanity check for the test above: confirms the preflight endpoint is
    // reachable with no Authorization header at all (it's intentionally
    // public, see backend/src/api/preflight.rs), so the api-key-scoped
    // rate-limit header assertion above is actually distinguishing
    // authenticated from anonymous calls, not just checking "is present".
    const res = await request.get(
      "/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
    );
    expect(res.ok(), await res.text()).toBeTruthy();
    // No bearer token → api_key_rate_limit_middleware is a no-op passthrough,
    // confirming the header in the test above really is API-key-specific.
    expect(res.headers()["ratelimit-limit"]).toBeFalsy();
  });
});
