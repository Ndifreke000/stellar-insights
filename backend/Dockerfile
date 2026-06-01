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

# -------- Runtime stage --------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r appuser && useradd -r -g appuser -d /app -s /sbin/nologin appuser

COPY --from=builder /app/target/release/stellar-insights-backend /usr/local/bin/stellar-insights-backend
COPY --from=builder /app/entrypoint.sh /app/entrypoint.sh
COPY --from=builder /app/scripts/run_migrations.sh /app/scripts/run_migrations.sh
COPY --from=builder /app/scripts/rollback_last_migration.sh /app/scripts/rollback_last_migration.sh
RUN chmod +x /app/entrypoint.sh /app/scripts/run_migrations.sh /app/scripts/rollback_last_migration.sh

USER appuser

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["/usr/local/bin/stellar-insights-backend"]
