# Data Quality Monitoring Design

## Goal
Prevent bad data (nulls, negative values, sudden zero-volume) from reaching users and on-chain snapshots.

## Data Model & Expectations
We will implement a lightweight, Great Expectations-style validation module in Rust.

### Expectations
- `NotNullExpectation`: Ensures a column (e.g., `asset_id`) is not NULL.
- `PositiveValueExpectation`: Ensures a numeric column (e.g., `amount`, `volume`) is > 0.
- `ZeroVolumeAnomalyExpectation`: Flags if a corridor's volume drops to exactly 0 when historical average > 0.

### Flow
1. Data is ingested and aggregated.
2. The `DataQualityService` is invoked.
3. If any **critical** expectation fails, the snapshot submission is blocked, and an alert is logged.
4. Dashboard (frontend) will query `GET /api/v1/health/data-quality` to display failures.

## Failure Modes
- **DB Connection Failure**: Log and fail closed (block snapshot).
- **Missing Expectations**: Default to pass, but log a warning.
