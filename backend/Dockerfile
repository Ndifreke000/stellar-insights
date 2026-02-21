# -------- Build stage --------
FROM rust:latest AS builder


WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY . .


# Copy migration/runner scripts
COPY scripts/run_migrations.sh /app/scripts/run_migrations.sh
COPY scripts/rollback_last_migration.sh /app/scripts/rollback_last_migration.sh
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/scripts/run_migrations.sh /app/scripts/rollback_last_migration.sh /app/entrypoint.sh

RUN cargo build --release

# Set entrypoint to run migrations then start app
ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["/app/target/release/stellar-insights-backend"]
