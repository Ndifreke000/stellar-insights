# Multi-stage build for production

# Backend build stage
FROM rust:1.70 as backend-builder

WORKDIR /app/backend
COPY backend/Cargo.* ./
COPY backend/src ./src

RUN cargo build --release

# Frontend build stage
FROM node:18-alpine as frontend-builder

WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci

COPY frontend . 
RUN npm run build

# Runtime stage
FROM ubuntu:22.04

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy backend binary from builder
COPY --from=backend-builder /app/backend/target/release/stellar-reserves-monitor /usr/local/bin/
COPY backend/.env /.env

# Copy frontend from builder
COPY --from=frontend-builder /app/frontend/.next ./frontend/.next
COPY --from=frontend-builder /app/frontend/node_modules ./frontend/node_modules
COPY --from=frontend-builder /app/frontend/package.json ./frontend/

EXPOSE 3000 3001

CMD ["sh", "-c", "stellar-reserves-monitor & cd frontend && npm start"]
