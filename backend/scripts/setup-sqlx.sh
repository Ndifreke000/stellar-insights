#!/bin/bash
# Setup script for SQLx compile-time verification
#
# The backend is SQLite-only (docs/adr/0001-sqlite-vs-postgres.md) --
# `sqlx` in Cargo.toml only enables the "sqlite" feature. This just needs a
# local SQLite file, not a database server.

set -e

echo "🔧 Setting up database for SQLx compile-time verification..."

DB_FILE="./payraider.db"

# Set DATABASE_URL
export DATABASE_URL="sqlite://${DB_FILE}"
echo "DATABASE_URL=$DATABASE_URL" > .env

echo "✅ Database URL set in .env file"

# Check if sqlx-cli is installed
if ! command -v sqlx &> /dev/null; then
    echo "📦 Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features sqlite
fi

# Create the database file if it doesn't exist yet
if [ ! -f "${DB_FILE}" ]; then
    echo "🗄️  Creating SQLite database at ${DB_FILE}..."
    sqlx database create
fi

# Run migrations if they exist
if [ -d "migrations" ]; then
    echo "🔄 Running database migrations..."
    sqlx migrate run
else
    echo "⚠️  No migrations directory found. You may need to create the database schema manually."
fi

# Generate SQLx prepared data
echo "📝 Generating SQLx prepared data..."
cargo sqlx prepare

echo "✅ Setup complete! You can now run:"
echo "   cargo build"
echo "   cargo clippy --all-targets --all-features"
echo "   cargo test"
