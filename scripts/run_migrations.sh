#!/bin/bash
# Run all pending database migrations using sqlx
# Usage: ./run_migrations.sh

set -e
cd "$(dirname "$0")"

if ! command -v cargo-sqlx &> /dev/null && ! cargo sqlx --version &> /dev/null; then
  echo "sqlx is not installed. Install with: cargo install sqlx-cli --no-default-features --features native-tls,sqlite"
  exit 1
fi

cd ../backend

cargo sqlx migrate run
