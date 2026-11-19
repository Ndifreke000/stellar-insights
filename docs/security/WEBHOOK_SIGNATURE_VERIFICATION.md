# Webhook Delivery Signature Verification

**Issue**: [#2578 — Verify webhook delivery signatures and document verification for consumers](https://github.com/Ndifreke000/stellar-insights/issues/2578)  
**Status**: Implementation required  
**Estimated size**: S (half a day to a day)

---

## Overview

Every outgoing webhook delivery is signed with an HMAC-SHA256 signature so consumers can verify the request originated from Stellar Insights and was not tampered with in transit. This document describes the current signing implementation, the signature format on the wire, the replay-protection window, and how consumers (and SDK users) should verify incoming webhooks.

---

## Threat Model

**Who is the attacker?** A network-level adversary or a malicious third party that can intercept or forge HTTP POST requests to a consumer's endpoint.

**What can they do today (without verification)?** Without signature verification a consumer has no cryptographic proof the request came from Stellar Insights. An attacker could:

- **Replay** a legitimate past delivery to trigger duplicate processing (e.g. crediting a payment twice).
- **Forge** a delivery with a fabricated payload, tricking consumer logic into acting on false data.
- **Tamper** with a legitimate delivery in transit and alter amounts, account IDs, or event types.

**What the fix closes**: HMAC-SHA256 signatures tied to a per-webhook secret and a delivery timestamp make forgery and replay computationally infeasible.

---

## Current Implementation

### Signing location

`backend/src/services/webhook_dispatcher.rs` — `deliver_webhook()`

The dispatcher constructs a JSON envelope, signs it, then sends it as the HTTP body:

```rust
// webhook_dispatcher.rs
let body = serde_json::to_string(&envelope)?;
let signature = WebhookSignature::sign(&body, secret);
```

### Signature algorithm

`backend/src/webhooks/mod.rs` — `WebhookSignature::sign()`

```rust
pub fn sign(payload: &str, secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    Mac::update(&mut mac, payload.as_bytes());
    format!("sha256={}", hex::encode(Mac::finalize(mac).into_bytes()))
}
```

- Algorithm: **HMAC-SHA256**
- Key: the per-webhook `secret` stored alongside the webhook registration
- Input: the **full serialised JSON body** (the complete envelope, not just the `data` field)
- Output format: `sha256=<lowercase-hex>`

### Delivery envelope

```json
{
  "id": "<uuid-v4>",
  "event": "<event_type>",
  "timestamp": 1700000000,
  "data": { ...event payload... }
}
```

### Request headers sent on every delivery

| Header | Value |
|---|---|
| `X-Zapier-Signature` | `sha256=<hmac_hex>` |
| `X-Zapier-Timestamp` | Unix timestamp (integer seconds, same value as `envelope.timestamp`) |
| `X-Zapier-Delivery-ID` | UUID v4 unique to this delivery attempt |
| `X-Zapier-Event` | Event type string (e.g. `payment.completed`) |
| `Content-Type` | `application/json` |

---

## Verification Steps for Consumers

Consumers **must** perform all three checks to be protected against forgery and replay.

### Step 1 — Replay check

Read the `X-Zapier-Timestamp` header and reject the request if the timestamp is more than **5 minutes** in the past or future:

```
abs(now_unix - X-Zapier-Timestamp) <= 300
```

Reject with HTTP 400 if outside the window.

### Step 2 — Recompute the signature

Recompute HMAC-SHA256 over the **raw request body bytes** (do not parse and re-serialise; byte order matters) using the shared secret:

```
expected = "sha256=" + hex( HMAC-SHA256(secret, raw_body_bytes) )
```

### Step 3 — Constant-time comparison

Compare `expected` against the `X-Zapier-Signature` header using a **constant-time equality function**. Do not use a regular `==` or `strcmp`; timing side-channels allow an attacker to brute-force the signature byte by byte.

---

## SDK Helpers

Both SDKs expose a `verifyWebhook` helper that encapsulates all three steps.

### TypeScript SDK

```typescript
import { verifyWebhook } from '@stellar-insights/sdk';

// In your Express/Next.js webhook handler:
app.post('/webhook', express.raw({ type: 'application/json' }), (req, res) => {
  const result = verifyWebhook({
    rawBody: req.body,           // Buffer — must be the raw bytes, not parsed JSON
    signature: req.headers['x-zapier-signature'] as string,
    timestamp: req.headers['x-zapier-timestamp'] as string,
    secret: process.env.WEBHOOK_SECRET!,
    toleranceSecs: 300,          // optional, defaults to 300
  });

  if (!result.valid) {
    return res.status(400).json({ error: result.reason });
  }

  // Safe to process
  const event = result.envelope; // typed WebhookEnvelope
  res.sendStatus(200);
});
```

**Important**: use `express.raw()` (or equivalent) to capture the body before any JSON parsing middleware runs. Parsing and re-serialising changes byte layout and will break signature verification.

### Python SDK

```python
from stellar_insights import verify_webhook
from flask import Flask, request, abort

app = Flask(__name__)

@app.route('/webhook', methods=['POST'])
def handle_webhook():
    result = verify_webhook(
        raw_body=request.get_data(),      # bytes — do not call .json() first
        signature=request.headers.get('X-Zapier-Signature', ''),
        timestamp=request.headers.get('X-Zapier-Timestamp', ''),
        secret=os.environ['WEBHOOK_SECRET'],
        tolerance_secs=300,
    )

    if not result.valid:
        abort(400, description=result.reason)

    event = result.envelope  # dict with id, event, timestamp, data
    return '', 200
```

### Manual verification (any language)

```python
import hmac, hashlib, time

def verify_webhook_manual(raw_body: bytes, signature: str,
                           timestamp: str, secret: str) -> bool:
    # Step 1: replay window
    if abs(time.time() - int(timestamp)) > 300:
        return False

    # Step 2: recompute
    expected = 'sha256=' + hmac.new(
        secret.encode(), raw_body, hashlib.sha256
    ).hexdigest()

    # Step 3: constant-time compare
    return hmac.compare_digest(expected, signature)
```

---

## Secret Management

- Each webhook registration gets a unique randomly-generated secret stored in the database alongside the webhook record.
- The secret is returned **once** at registration time in the `POST /api/v1/webhooks` response and is never returned again.
- Treat the secret like a password: store it in an environment variable or secrets manager, never in source code.
- To rotate a secret, delete the webhook and re-register it. A dedicated rotate endpoint is tracked in [#2578](https://github.com/Ndifreke000/stellar-insights/issues/2578).

---

## Retry Behaviour

If your endpoint returns a non-2xx status the dispatcher retries delivery up to **3 times** with exponential backoff (base delay 500 ms). Each retry is a new HTTP POST with a new `X-Zapier-Delivery-ID` but the **same** `X-Zapier-Timestamp` as the original attempt. Implement idempotency on `X-Zapier-Delivery-ID` to handle retries safely.

| Attempt | Delay before retry |
|---|---|
| 1st retry | ~500 ms |
| 2nd retry | ~1 000 ms |
| 3rd retry | ~2 000 ms |
| After 3rd | Event marked `failed`, no more retries |

---

## Open Items and Known Gaps

| Gap | Severity | Tracked in |
|---|---|---|
| Dispatcher does not re-validate the webhook URL at send time (only at registration) | Medium | #2579 |
| No dedicated secret-rotation endpoint; consumers must delete and re-register to rotate | Low | #2578 |
| Header prefix `X-Zapier-*` is misleading for non-Zapier consumers; rename to `X-Stellar-Insights-*` | Low | #2578 |
| Redirect following is not disabled on the dispatcher HTTP client (up to 10 hops by default) | Medium | #2579 |

---

## Related Documentation

- [Webhook SSRF Protection](./WEBHOOK_SSRF_PROTECTION.md) — issue #2579
- [API Request Signing](../API_REQUEST_SIGNING.md) — inbound request signing for API callers
- `backend/src/webhooks/mod.rs` — `WebhookSignature` struct
- `backend/src/services/webhook_dispatcher.rs` — `deliver_webhook()`
- `backend/src/api/webhooks.rs` — registration, ownership, and URL validation
