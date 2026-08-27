# QA: SEP-10 auth demo (`/[locale]/sep10-demo`) — #1814

Manual browser verification that the demo completes a full SEP-10 challenge / sign / verify
round trip against the backend, rather than just rendering a static form.

Setup and reporting conventions: [README](README.md).

## What the code does

[`sep10-demo/page.tsx`](../../frontend/src/app/%5Blocale%5D/sep10-demo/page.tsx) is a thin
UI over `useWallet()` from
[`wallet-context.tsx`](../../frontend/src/components/lib/wallet-context.tsx). The page owns
almost no logic: two status cards, three buttons, and a result panel. The round trip lives
in [`services/sep10Auth.ts`](../../frontend/src/services/sep10Auth.ts).

`authenticateWithSep10` calls `sep10AuthService.authenticate(address, { homeDomain: window.location.hostname })`,
then stores the token and an expiry (`Date.now() + expires_in * 1000`) in `localStorage`.

### Network calls

Prefixed with `NEXT_PUBLIC_API_URL`:

| Step | Request |
| --- | --- |
| Request challenge | `POST {API}/api/sep10/auth` |
| Verify signed challenge | `POST {API}/api/sep10/verify` |
| Logout | `POST {API}/api/sep10/logout` (Bearer token) |
| Server info | `GET {API}/api/sep10/info` |

Signing happens in the wallet extension between `auth` and `verify` — no network call, but
it is where the flow most often stalls.

The backend handlers exist in [`backend/src/api/sep10.rs`](../../backend/src/api/sep10.rs)
and match these paths.

## Blockers found by code read

Confirm against the running app before filing.

### B1 — SEP-10 routes are not mounted

`sep10::routes()` (defined at the bottom of `backend/src/api/sep10.rs`) is referenced only
from [`backend/src/openapi.rs`](../../backend/src/openapi.rs). The app router in
[`backend/src/api/v1/mod.rs`](../../backend/src/api/v1/mod.rs) never merges it.

The issue notes the `sep10_integration` tests were fixed this session. Those tests exercise
the handlers; they do not prove the handlers are reachable over HTTP on the running server.
That gap is exactly what this QA pass is for.

```bash
curl -i localhost:8080/api/sep10/info
curl -i -X POST localhost:8080/api/sep10/auth \
  -H 'content-type: application/json' \
  -d '{"account":"GXXXX..."}'
```

Expect 404. Do this **before** opening the browser — it takes ten seconds and tells you
whether the rest of the plan can run.

### B2 — `/api/protected-endpoint` does not exist

The "Test Authenticated Request" button fetches a **relative** `/api/protected-endpoint`
([page.tsx:35](../../frontend/src/app/%5Blocale%5D/sep10-demo/page.tsx#L35)) — relative, so
it hits the Next dev server, not the backend. There is no such route:
[`frontend/src/app/api/`](../../frontend/src/app/api/) contains `dashboard`, `error-log`,
`example`, `metrics`, and `network-graph` only.

Predicted result: 404, rendered by the page's own handling as
`Failed: 404 Not Found` in the red result panel. That is at least a readable message rather
than a raw object, but the button cannot demonstrate what it claims to. File it as a bug —
either the route is missing or the button should point at a real protected backend
endpoint.

## Test steps

### 1. Page loads

1. Navigate to `/en/sep10-demo` with Console and Network open.
2. Header, both status cards, and the explanatory sections render.
3. Wallet card shows "Not Connected"; auth card shows "Not Authenticated".
4. Only "Step 1: Connect Wallet" is visible.

Note: this page's markup is styled independently of the rest of the app (hard-coded
`bg-white dark:bg-gray-800`, `text-gray-900` rather than the design tokens the other routes
use). If it looks visually out of place next to `/en/trustlines`, that is real and worth a
low-priority bug — but do not block the functional pass on it.

### 2. Connect wallet

1. Click "Step 1: Connect Wallet".
2. The wallet extension prompts; approve.
3. Wallet card flips to "Connected" with a green check.
4. Your `G...` address renders below it, in full.
5. "Step 2: Authenticate with SEP-10" appears; step 1 disappears.

**Test the rejection path too:** reload, click connect, and **decline** in the wallet. The
page's `connectWallet` rethrows and the page has no catch on the click handler — check for
an unhandled rejection in the console and whether the UI stays stuck. Expect the card to
remain "Not Connected" and no console error; anything else is a bug.

Also test with **no wallet extension installed** (a fresh browser profile). The context
throws `No compatible Stellar wallet found. Please install Freighter, Albedo, xBull, or
Rabet.` — confirm the user actually sees that string somewhere, not just the console.

### 3. Challenge / sign / verify round trip

This is the core of #1814. Watch the Network tab throughout.

1. Click "Step 2: Authenticate with SEP-10".
2. **`POST /api/sep10/auth` fires.** Confirm:
   - status 200;
   - the request body carries your account;
   - the response contains a `transaction` (challenge XDR) and `network_passphrase`.
3. Confirm the `network_passphrase` matches your wallet's network (testnet:
   `Test SDF Network ; September 2015`). A mismatch here is the single most common cause of
   a wallet refusing to sign, and it will surface as a wallet-side error with no obvious
   cause.
4. The wallet prompts to sign the challenge. Approve.
5. **`POST /api/sep10/verify` fires.** Confirm status 200 and a `token` + `expires_in` in
   the response.
6. Auth card flips to "Authenticated"; the truncated token renders (`Token: <20 chars>...`).
7. "Test Authenticated Request" and "Logout" appear.

**Expected (blocker):** with B1, step 2 returns 404 and the flow stops there.

Note `homeDomain` is `window.location.hostname` — `localhost` in dev. If the backend
validates the home domain against a configured value, this fails at `verify` even when
`auth` succeeds. If you get a 200 on `auth` and a 4xx on `verify`, check the response body
for a domain mismatch before assuming the signature is at fault.

### 4. Token persistence

1. With the session authenticated, inspect `localStorage`: an auth token and a token
   expiry timestamp are present.
2. Reload the page. Confirm whether the authenticated state is restored — and if it is,
   that the restored state is consistent (card says Authenticated **and** the token is
   actually still valid).
3. Manually set the stored expiry to a past timestamp and reload. The page must not present
   an expired token as a live session.

### 5. Authenticated request

1. Click "Test Authenticated Request".
2. Confirm the request carries `Authorization: Bearer <token>`.

**Expected (blocker):** 404 per B2, rendered as `Failed: 404 Not Found` in the red panel.
Screenshot it and file.

### 6. Logout

1. Click "Logout".
2. `POST /api/sep10/logout` fires with the Bearer token; status 200.
3. Both `localStorage` entries are removed.
4. Auth card returns to "Not Authenticated"; step 2 button reappears.
5. Confirm the wallet stays connected — logout should clear auth, not the connection.

Note `logout` swallows a failed server call and clears local state regardless. Force a
failure (stop the backend, then click Logout) and confirm the UI still returns to a clean
signed-out state rather than getting stuck.

### 7. Console and network audit

- Zero console errors on the happy path; zero unhandled rejections on the rejection paths.
- No full JWT in a URL — it belongs in the `Authorization` header. Check request URLs.
- The full token must not be rendered in the DOM beyond the intended 20-char preview.

## Acceptance criteria

- [ ] `/en/sep10-demo` loads with no console errors
- [ ] Wallet connects, and both rejection and no-wallet paths render meaningful messages
- [ ] `POST /api/sep10/auth` returns 200 with a challenge XDR (B1 resolved)
- [ ] The wallet signs the challenge
- [ ] `POST /api/sep10/verify` returns 200 with a session token
- [ ] The UI reflects the authenticated state and stores the token with an expiry
- [ ] Expired-token state is handled on reload
- [ ] The authenticated-request button hits a route that exists (B2 resolved)
- [ ] Logout clears state and returns the UI to signed-out, even when the server call fails
- [ ] Any failure has a follow-up bug filed and linked from #1814

## Results

| Step | Result | Notes / bug link |
| --- | --- | --- |
| 1. Page loads | | |
| 2. Connect wallet | | |
| 2b. Reject / no wallet | | |
| 3. Challenge → sign → verify | | |
| 4. Token persistence | | |
| 5. Authenticated request | | |
| 6. Logout | | |
| 7. Console / network clean | | |

Tester: &nbsp; Date: &nbsp; Frontend SHA: &nbsp; Backend SHA:
