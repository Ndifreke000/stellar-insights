# Account Merge Detection

This feature detects Stellar `account_merge` operations during ledger ingestion, stores merge events, and exposes analytics endpoints for merge behavior.

## Data Captured

Each merge event stores:
- `operation_id`
- `transaction_hash`
- `ledger_sequence`
- `source_account` (merged account)
- `destination_account` (recipient account)
- `merged_balance` (credited amount resolved from operation effects)
- `created_at`

## Ingestion Flow

1. Ledger ingestion fetches operations for each ingested ledger.
2. `account_merge` operations are filtered.
3. Effects are fetched per merge operation to resolve credited amount.
4. Merge events are inserted into `account_merges` (deduplicated by `operation_id`).

## API Endpoints

### `GET /api/account-merges/stats`
Returns aggregate metrics:
- total merges
- total merged balance
- unique source accounts
- unique destination accounts

### `GET /api/account-merges/recent?limit=50`
Returns the most recent merge events.
- `limit` min: 1
- `limit` max: 200

### `GET /api/account-merges/destinations?limit=20`
Returns destination-account patterns ordered by merge frequency and merged balance.
- `limit` min: 1
- `limit` max: 100

## Database Migration

`backend/migrations/012_create_account_merges.sql` creates:
- `account_merges` table
- indexes on ledger sequence, source account, destination account, and timestamp
