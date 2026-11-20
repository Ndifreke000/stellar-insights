#!/bin/bash
# Run database migrations as a separate job
#
# This script runs migrations using sqlx and is intended to be called
# from CI/CD pipelines or manually before deploying new versions.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Load environment variables
if [[ -f .env ]]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Database URL (SQLite by default)
DB_URL="${DATABASE_URL:-sqlite::memory:}"

echo "Running database migrations..."
echo "Database: $DB_URL"
echo ""

# Check if sqlx is available
if ! command -v sqlx &> /dev/null; then
    echo "sqlx not found. Installing..."
    cargo install sqlx-cli --no-default-features --features sqlite
fi

# Run migrations
if [[ "$DB_URL" == *"sqlite"* ]]; then
    echo "Running migrations on SQLite database..."
    sqlx migrate run --database-url "$DB_URL"
elif [[ "$DB_URL" == *"postgres"* ]]; then
    echo "Running migrations on PostgreSQL database..."
    sqlx migrate run --database-url "$DB_URL"
else
    echo "Unsupported database: $DB_URL"
    echo "Supported databases: SQLite, PostgreSQL"
    exit 1
fi

echo ""
echo "✅ Migrations completed successfully!"
