# TODO - Surface Soroban contract metadata in analytics and validation flows

## Step 1 - Locate integration points (approved)
- [ ] Read backend `EventIndexer` implementation and related services used by `api/contract_events.rs`.
- [ ] Identify existing DB schema / models for contract verification, asset verification, and governance validation.
- [ ] Identify whether there is already an API response shape for “validation status” that we can extend.

## Step 2 - Backend: ingest/validate Soroban contract metadata
- [ ] Add/extend a backend service to fetch Soroban contract metadata via RPC.
- [ ] Parse only the fields needed for validation/mismatch checks.
- [ ] Store ingestion state (ok/mismatch/unsupported/stale + timestamps + error reason).

## Step 3 - Backend: expose validation/mismatch results
- [ ] Extend existing endpoints (likely `/api/analytics/verification-summary` or contract-events endpoints) to include metadata validation status.
- [ ] Keep changes backward-compatible (add optional fields).

## Step 4 - Frontend: display warnings/mismatches
- [ ] Locate UI components that currently show verification status.
- [ ] Extend UI to display metadata validation status + mismatch warnings.
- [ ] Ensure graceful fallback when metadata info is missing.

## Step 5 - Documentation
- [ ] Document integration flow for contributors: fetch -> parse -> store -> validate -> UI.
- [ ] Mention unsupported/stale behavior and how to troubleshoot.

## Step 6 - Verification
- [ ] Run backend tests/lints (as configured).
- [ ] Run frontend typecheck/tests.
- [ ] Sanity-check API schema compatibility.
