# Muxed Accounts (M-addresses) Support

This document describes muxed account support in Stellar Insights.

## Overview

**Muxed accounts** (M-addresses) allow a single Stellar account (G-address) to represent multiple sub-accounts via a 64-bit muxed ID. They are defined in [SEP-0023](https://stellar.org/protocol/sep-23) and the [Stellar Muxed Accounts FAQ](https://stellar.org/blog/developers/muxed-accounts-faq).

- **G-address**: 56 characters, starts with `G` (base account).
- **M-address**: 69 characters, starts with `M` (base account + muxed ID encoded).

## Backend

### Module: `muxed` (`backend/src/muxed.rs`)

- **`is_muxed_address(addr)`** – Returns true if the string is a valid M-address format (69 chars, starts with `M`).
- **`is_stellar_account_address(addr)`** – True for G or M.
- **`parse_muxed_address(addr)`** – Parses an M-address into `MuxedAccountInfo`: optional base G-address and muxed ID. Decoding is best-effort (depends on strkey compatibility).
- **`normalize_account_input(addr)`** – Validates and trims G or M for storage/API.

### Models (`backend/src/models.rs`)

- **`MuxedAccountAnalytics`** – Aggregated muxed usage: total muxed payments, unique M-addresses, top by activity, base accounts with muxed.
- **`MuxedAccountUsage`** – Per-address: `account_address`, `base_account`, `muxed_id`, payment counts as source/destination.

### Storage

- **Payments**: `source_account` and `destination_account` in the `payments` table store raw addresses (G or M). No schema change required.
- **Analytics**: `GET /api/analytics/muxed?limit=20` computes from `payments`: counts where source or destination is M-address, and top M-addresses by activity.

### API

- **`GET /api/analytics/muxed`** – Returns muxed account analytics (query param `limit`, default 20, max 100).
- **`GET /api/anchors/account/:stellar_account`** – Accepts G or M. If M, the backend resolves to the base account for anchor lookup.

## Frontend

### Address utilities (`frontend/src/lib/address.ts`)

- **`isMuxedAddress(addr)`** – M-address format check.
- **`isGAddress(addr)`** – G-address format check.
- **`isStellarAccountAddress(addr)`** – G or M.
- **`formatAddressShort(address, startChars, endChars)`** – Truncate for display (works for both G and M).
- **`getAddressValidationError(addr)`** – Validation message for inputs.

### API (`frontend/src/lib/api.ts`)

- **`getAnchorDetail(address)`** – If `address` is G or M, calls `GET /api/anchors/account/:address`; otherwise uses anchor ID.
- **`getMuxedAnalytics(limit?)`** – Fetches `GET /api/analytics/muxed`.

### UI

- **Anchor detail** (`/anchors/[address]`): Accepts G or M in the URL; validation uses `getAddressValidationError`.
- **Address displays**: Anchors table and related components use `formatAddressShort` for consistent truncation of G and M.
- **Analytics**: The **Muxed Accounts** card on the analytics page shows total muxed payments, unique M-addresses, and top M-addresses by activity.

## Acceptance Criteria (from issue)

| Criterion | Status |
|-----------|--------|
| Parse muxed account addresses | ✅ Backend `muxed` module + frontend `address.ts` |
| Store muxed account data | ✅ Payments store raw M-addresses in `source_account` / `destination_account` |
| Display M-addresses in UI | ✅ `formatAddressShort` and address validation for G/M |
| Track muxed account usage | ✅ Analytics from `payments` table |
| Muxed account analytics | ✅ `GET /api/analytics/muxed` + MuxedAccountCard on analytics page |
| Update all address displays | ✅ Anchors table, anchor detail, analytics card use shared formatting/validation |
| M-address input in UI | ✅ Anchor detail accepts G or M; validation allows both |
| Tests and documentation | ✅ Unit tests in `muxed.rs`; this doc |

## Labels

`enhancement` · `stellar` · `muxed-accounts`
