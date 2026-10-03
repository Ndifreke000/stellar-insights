# PayRaider backend image.
#
#   docker build -t payraider-backend backend
#
# Migrations run inside the app at startup (sqlx::migrate!), so the image
# needs no migration tooling. The SQLite file should live on a volume:
#   DATABASE_URL=sqlite:///data/payraider.db?mode=rwc  with /data mounted.

# -------- Build stage --------
FROM rust:1-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY . .
# Compile-time query checks use the checked-in .sqlx cache (see .cargo/config.toml).
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked --bin payraider-backend

# -------- Runtime stage --------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

RUN addgroup --system --gid 1001 payraider && \
    adduser --system --uid 1001 --gid 1001 --home /app --no-create-home payraider && \
    mkdir -p /data && chown payraider:payraider /data

COPY --from=builder /app/target/release/payraider-backend /usr/local/bin/payraider-backend

USER payraider
WORKDIR /app
ENV RUST_LOG=info \
    LOG_FORMAT=json \
    DATABASE_URL=sqlite:///data/payraider.db?mode=rwc

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=3 \
    CMD curl -fsS "http://localhost:${SERVER_PORT:-${PORT:-8080}}/health" || exit 1

# Hosts such as Railway assign the port in $PORT; SERVER_PORT wins if set.
CMD ["/bin/sh", "-c", "SERVER_PORT=\"${SERVER_PORT:-${PORT:-8080}}\" exec /usr/local/bin/payraider-backend"]
