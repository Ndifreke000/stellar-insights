#!/bin/bash
# Setup script for SQLx compile-time verification

set -e

echo "🔧 Setting up database for SQLx compile-time verification..."

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

# Check if PostgreSQL container is already running
if docker ps | grep -q stellar-postgres; then
    echo "✅ PostgreSQL container is already running"
else
    echo "🐳 Starting PostgreSQL container..."
    docker run --name stellar-postgres \
      -e POSTGRES_PASSWORD=password \
      -e POSTGRES_DB=stellar_insights \
      -p 5432:5432 -d postgres:14
    
    echo "⏳ Waiting for PostgreSQL to be ready..."
    sleep 5
fi

# Set DATABASE_URL
export DATABASE_URL="postgresql://postgres:password@localhost:5432/stellar_insights"
echo "DATABASE_URL=$DATABASE_URL" > .env

echo "✅ Database URL set in .env file"

# Check if sqlx-cli is installed
if ! command -v sqlx &> /dev/null; then
    echo "📦 Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres,sqlite
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
