# Backend Rust Style Guide

## Naming Conventions

### Variables
- Use descriptive snake_case names: `corridor_health_score` not `chs`.
- Prefer fully spelled names: `transaction_count` not `tx_cnt`.
- Accepted abbreviations: `id`, `url`, `api`, `rpc`, `usd`.

### Functions
- Use snake_case verb phrases: `calculate_health_score()`, `fetch_corridor_data()`.
- Prefer precise names: `fetch_corridor_by_key()` over `get_corridor()`.

### Types
- Use PascalCase noun phrases: `CorridorMetrics`, `PaymentProcessor`.
- Avoid stuttering in module contexts.

### Constants
- Use SCREAMING_SNAKE_CASE: `MAX_RETRIES`, `DEFAULT_TIMEOUT_MS`.

### Modules
- Use snake_case module names.

### Traits
- Use PascalCase trait names.

## Abbreviations

### Allowed
- `id`
- `url`
- `api`
- `rpc`
- `usd`

### Avoid
- `tx` (use `transaction`)
- `msg` (use `message`)
- `cfg` (use `config`)
- `ctx` (use `context`)
