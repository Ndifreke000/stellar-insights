# SEP-6 Deposit & Withdrawal UI

This document describes the SEP-6 (Deposit and Withdrawal API) user interface in Stellar Insights.

## Overview

SEP-6 is the Stellar standard for **programmatic** deposit and withdrawal: the client requests instructions (e.g. “how to deposit”) via GET parameters and the anchor returns type, URL, or transaction id. This is in contrast to SEP-24, which uses interactive hosted flows.

The UI provides:

- **Anchor selection** – Preset list from backend or custom transfer server URL
- **Deposit form** – Asset, Stellar account (G or M), amount, memo, email
- **Withdrawal form** – Asset, withdrawal type, account, destination, amount, memo
- **Transaction status** – Look up by ID and list recent transactions (when JWT is provided)

## Files

| File | Purpose |
|------|---------|
| `frontend/src/app/sep6/page.tsx` | SEP-6 page: anchor selection, deposit/withdraw tabs, transaction status |
| `frontend/src/components/Sep6DepositForm.tsx` | Deposit request form and result display |
| `frontend/src/components/Sep6WithdrawForm.tsx` | Withdrawal request form and result display |
| `frontend/src/services/sep6.ts` | SEP-6 API client (info, deposit, withdraw, transaction, transactions) |

## API Assumptions

The frontend calls a **backend proxy** at `NEXT_PUBLIC_API_URL` (e.g. `http://127.0.0.1:8080`) under the path `/api/sep6/*`. The backend must:

1. Proxy requests to the anchor’s transfer server (SEP-6 base URL).
2. Forward query parameters (e.g. `transfer_server`, `asset_code`, `account`, `type` for withdraw).
3. Optionally forward `jwt` for authenticated endpoints (e.g. `GET /transactions`).

Expected proxy endpoints:

- `GET /api/sep6/anchors` – List of SEP-6 anchors (name, transfer_server).
- `GET /api/sep6/info?transfer_server=...` – Anchor capabilities (deposit/withdraw methods).
- `GET /api/sep6/deposit?transfer_server=...&asset_code=...&account=...&...` – Deposit instructions.
- `GET /api/sep6/withdraw?transfer_server=...&asset_code=...&type=...&account=...&...` – Withdrawal instructions.
- `GET /api/sep6/transaction?transfer_server=...&id=...&jwt=...` – Single transaction status.
- `GET /api/sep6/transactions?transfer_server=...&kind=...&jwt=...` – List transactions.

If the backend proxy is not implemented yet, the UI will load but requests will fail (e.g. “Failed to load anchors”).

## Validation

- **Stellar account**: G-address (56 chars) or M-address (69 chars) via `getAddressValidationError` from `@/lib/address`.
- **Required fields**: Asset and account for both flows; withdrawal type for withdraw.
- **Optional**: Amount, memo, memo_type, email (deposit), dest, dest_extra (withdraw).

## Navigation

- **Sidebar**: “SEP-6” links to `/sep6`.
- Direct URL: `/sep6`.

## Acceptance Criteria

| Criterion | Status |
|-----------|--------|
| Create deposit form | ✅ `Sep6DepositForm.tsx` |
| Create withdrawal form | ✅ `Sep6WithdrawForm.tsx` |
| Display transaction status | ✅ Look up by ID + recent list on page |
| Support multiple anchors | ✅ Preset list + custom URL |
| Add validation | ✅ Account format, required fields |
| Add tests and documentation | ✅ This doc; unit/integration tests can be added (e.g. Jest/Vitest) |

## Labels

`enhancement` · `stellar` · `sep-6` · `ui`
