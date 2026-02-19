# Backend - Analytics Engine (Rust)

This is the core analytics engine that powers Stellar Insights. It ingests blockchain data and computes reliability metrics for payment corridors.

## Quick Start

### Prerequisites

- Rust 1.70+ ([install](https://rustup.rs/))
- PostgreSQL 14+ (or Docker)
- Copy `.env.example` to `.env`

### Setup

1. **Start PostgreSQL**
```bash
# Using Docker
docker run --name stellar-postgres \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=stellar_insights \
  -p 5432:5432 \
  -d postgres:14
```

2. **Configure Environment**
```bash
cp .env.example .env
# Edit .env if using different database credentials
```

3. **Run Server**
```bash
cargo run
```

You should see:
```
INFO backend: Connecting to database...
INFO backend: Running database migrations...
INFO backend: Server starting on 127.0.0.1:8080
```

## API Endpoints

### Health Check
```bash
GET /health
```

### Anchors
```bash
# List all anchors
GET /api/anchors

# Create anchor
POST /api/anchors
Content-Type: application/json
{
  "name": "Circle",
  "stellar_account": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
  "home_domain": "circle.com"
}

# Get anchor details
GET /api/anchors/:id

# Update anchor metrics
PUT /api/anchors/:id/metrics
{
  "total_transactions": 1000,
  "successful_transactions": 990,
  "avg_settlement_time_ms": 2000,
  "volume_usd": 100000.00
}

# List anchor assets
GET /api/anchors/:id/assets

# Add asset to anchor
POST /api/anchors/:id/assets
{
  "asset_code": "USDC",
  "asset_issuer": "GBBD47UZQ2YPJRYY34M5G5GRSTQ4OJIUJMRWP5EU7GRHST3DYKU6RVJ"
}
```

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_create_anchor
```

## Database Migrations

Migrations run automatically on server startup. To manually run them:

```bash
sqlx migrate run
```

To reset the database (WARNING: Deletes all data):
```bash
docker exec -it stellar-postgres \
  psql -U postgres -d stellar_insights -c \
  "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
```

## Project Structure

```
src/
├── main.rs              # Server entry point
├── lib.rs               # Library root
├── handlers.rs          # API request handlers
├── models.rs            # Data structures
├── database.rs          # Database operations
├── analytics.rs         # Metric computations
├── api/                 # API route modules
│   ├── anchors.rs       # Anchor endpoints
│   └── mod.rs
├── db/                  # Database layer
│   ├── aggregates.rs    # Aggregate queries
│   └── mod.rs
└── analytics/           # Analytics modules
    └── corridor.rs      # Corridor metrics
```

## Environment Variables

```
DATABASE_URL                # PostgreSQL connection string
RUST_LOG                   # Log level (info, debug, trace)
SERVER_HOST                # Server bind address (default: 127.0.0.1)
SERVER_PORT                # Server port (default: 8080)

# Graceful Shutdown Configuration (in seconds)
SHUTDOWN_GRACEFUL_TIMEOUT   # Max time for in-flight requests (default: 30)
SHUTDOWN_BACKGROUND_TIMEOUT # Max time for background tasks (default: 10)
SHUTDOWN_DB_TIMEOUT         # Max time for DB closure (default: 5)
```

## Graceful Shutdown

The backend implements comprehensive graceful shutdown handling. When receiving SIGTERM or SIGINT (Ctrl+C), it will:

1. Stop accepting new connections
2. Wait for in-flight requests to complete (with timeout)
3. Shutdown background tasks cleanly
4. Flush caches
5. Close database connections properly

See [GRACEFUL_SHUTDOWN.md](./GRACEFUL_SHUTDOWN.md) for detailed documentation.

### Quick Test

```bash
# Start server
cargo run

# In another terminal, send SIGTERM
kill -TERM $(pgrep -f "cargo run")

# Or just press Ctrl+C in the server terminal
# Watch the logs for graceful shutdown sequence
```

## Troubleshooting

### Database Connection Failed
```bash
# Check if PostgreSQL is running
docker ps | grep stellar-postgres

# Check logs
docker logs stellar-postgres

# Restart
docker restart stellar-postgres
```

### Port Already in Use
```bash
# Find what's using port 8080
lsof -i :8080

# Use different port
echo "SERVER_PORT=8081" >> .env
```

### Build Errors
```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build
```

## Development Notes

- Database migrations use SQLx for compile-time checking
- Async runtime: Tokio
- Web framework: Axum
- All timestamps are stored in UTC

## Contributing

When adding new endpoints:
1. Add handler to `handlers.rs`
2. Add routes to main route setup
3. Add database functions to `db/` modules
4. Add tests in `tests/` directory
5. Update this README

## Production Deployment

```bash
# Build optimized binary
cargo build --release

# Binary located at: target/release/backend
```

Run with production settings:
```bash
RUST_LOG=warn ./target/release/backend
```
