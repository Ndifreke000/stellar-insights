# Environment Configuration Guide

This guide explains how to configure the Stellar Insights backend using environment variables.

## Quick Start

1. Copy the example environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` with your configuration:
   ```bash
   # Use your preferred editor
   nano .env
   # or
   code .env
   ```

3. Start the application:
   ```bash
   cargo run
   ```

## Security Best Practices

### ⚠️ CRITICAL SECURITY RULES

1. **NEVER commit `.env` to version control**
   - The `.env` file is already in `.gitignore`
   - Always use `.env.example` for templates

2. **Use strong credentials in production**
   - Generate random passwords for databases
   - Use secure API keys
   - Rotate credentials regularly

3. **Restrict file permissions**
   ```bash
   chmod 600 .env
   ```

4. **Use secrets management in production**
   - AWS Secrets Manager
   - HashiCorp Vault
   - Kubernetes Secrets
   - Azure Key Vault

## Required Environment Variables

### DATABASE_URL (Required)

Database connection string.

**Development (SQLite):**
```bash
DATABASE_URL=sqlite:./stellar_insights.db
```

**Production (PostgreSQL):**
```bash
DATABASE_URL=postgresql://username:password@host:5432/database_name
```

**Security Notes:**
- Use strong passwords (16+ characters, mixed case, numbers, symbols)
- Never use default credentials in production
- Consider using connection pooling parameters

## Optional Environment Variables

### Server Configuration

```bash
# Host to bind to (default: 127.0.0.1)
SERVER_HOST=127.0.0.1

# Port to listen on (default: 8080)
SERVER_PORT=8080

# Logging level (default: info)
# Options: trace, debug, info, warn, error
RUST_LOG=info
```

### Redis Configuration

```bash
# Redis connection URL (default: redis://127.0.0.1:6379)
REDIS_URL=redis://127.0.0.1:6379

# For Redis with authentication:
# REDIS_URL=redis://username:password@host:6379

# For Redis with TLS:
# REDIS_URL=rediss://host:6380
```

### Database Connection Pool

```bash
# Maximum number of connections (default: 10)
DB_POOL_MAX_CONNECTIONS=10

# Minimum idle connections (default: 2)
DB_POOL_MIN_CONNECTIONS=2

# Connection timeout in seconds (default: 30)
DB_POOL_CONNECT_TIMEOUT_SECONDS=30

# Idle connection timeout in seconds (default: 600)
DB_POOL_IDLE_TIMEOUT_SECONDS=600

# Maximum connection lifetime in seconds (default: 1800)
DB_POOL_MAX_LIFETIME_SECONDS=1800
```

See [DATABASE_POOL_CONFIG.md](./DATABASE_POOL_CONFIG.md) for detailed pool configuration.

### Stellar Network Configuration

```bash
# Network to use: mainnet or testnet (default: mainnet)
STELLAR_NETWORK=mainnet

# Mainnet endpoints
STELLAR_RPC_URL_MAINNET=https://stellar.api.onfinality.io/public
STELLAR_HORIZON_URL_MAINNET=https://horizon.stellar.org

# Testnet endpoints
STELLAR_RPC_URL_TESTNET=https://soroban-testnet.stellar.org
STELLAR_HORIZON_URL_TESTNET=https://horizon-testnet.stellar.org

# Use mock RPC for testing (default: false)
RPC_MOCK_MODE=false
```

### Price Feed Configuration

```bash
# Price feed provider (default: coingecko)
PRICE_FEED_PROVIDER=coingecko

# API key for price feed (optional, improves rate limits)
PRICE_FEED_API_KEY=your_api_key_here

# Cache TTL in seconds (default: 900)
PRICE_FEED_CACHE_TTL_SECONDS=900

# Request timeout in seconds (default: 10)
PRICE_FEED_REQUEST_TIMEOUT_SECONDS=10
```

### CORS Configuration

```bash
# Comma-separated list of allowed origins
# Development:
CORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:3001

# Production (use your actual domains):
# CORS_ALLOWED_ORIGINS=https://stellar-insights.com,https://www.stellar-insights.com

# WARNING: Never use "*" in production!
```

### Compression Configuration

```bash
# Minimum response size in bytes to compress (default: 1024)
COMPRESSION_MIN_SIZE=1024
```

### Backup Configuration (Production Only)

```bash
# S3 bucket for backups
BACKUP_S3_BUCKET=your-backup-bucket-name

# Backup retention in days
BACKUP_RETENTION_DAYS=30

# Notification email for backup alerts
NOTIFICATION_EMAIL=admin@example.com

# WAL-G S3 prefix
WALG_S3_PREFIX=s3://your-backup-bucket-name/backups/

# PostgreSQL data directory
PGDATA=/var/lib/postgresql/data
```

## Environment-Specific Configurations

### Development

```bash
DATABASE_URL=sqlite:./stellar_insights.db
RUST_LOG=debug
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
REDIS_URL=redis://127.0.0.1:6379
RPC_MOCK_MODE=true
STELLAR_NETWORK=testnet
CORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:3001
DB_POOL_MAX_CONNECTIONS=5
```

### Staging

```bash
DATABASE_URL=postgresql://user:password@staging-db:5432/stellar_insights
RUST_LOG=info
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
REDIS_URL=redis://staging-redis:6379
RPC_MOCK_MODE=false
STELLAR_NETWORK=testnet
CORS_ALLOWED_ORIGINS=https://staging.stellar-insights.com
DB_POOL_MAX_CONNECTIONS=10
```

### Production

```bash
DATABASE_URL=postgresql://user:strong_password@prod-db:5432/stellar_insights
RUST_LOG=warn
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
REDIS_URL=redis://prod-redis:6379
RPC_MOCK_MODE=false
STELLAR_NETWORK=mainnet
CORS_ALLOWED_ORIGINS=https://stellar-insights.com,https://www.stellar-insights.com
DB_POOL_MAX_CONNECTIONS=20
PRICE_FEED_API_KEY=your_production_api_key
```

## Validation

The application validates environment variables on startup:

1. **Required variables** - Application fails if missing
2. **Format validation** - Checks ports, numbers, etc.
3. **Sanitized logging** - Credentials are redacted in logs

Example startup output:
```
INFO Starting Stellar Insights Backend
INFO Environment configuration:
INFO   DATABASE_URL: postgresql://user:****@localhost:5432/stellar_insights
INFO   SERVER_HOST: 127.0.0.1
INFO   SERVER_PORT: 8080
INFO   REDIS_URL: redis://****@localhost:6379
INFO   PRICE_FEED_API_KEY: [REDACTED]
```

## Troubleshooting

### Missing Required Variable

```
Error: Environment configuration errors:
  - Missing required environment variable: DATABASE_URL
```

**Solution:** Add the missing variable to your `.env` file.

### Invalid Port Number

```
Error: Environment configuration errors:
  - Invalid value for environment variable SERVER_PORT: 'abc'
```

**Solution:** Use a valid port number (1-65535).

### Database Connection Failed

```
Error: error connecting to database: Connection refused
```

**Solutions:**
- Verify database is running
- Check DATABASE_URL is correct
- Ensure network connectivity
- Check firewall rules

### Redis Connection Failed

```
WARN Failed to connect to Redis, refresh tokens will not persist
```

**Solutions:**
- Verify Redis is running
- Check REDIS_URL is correct
- Application will continue without Redis (degraded functionality)

## Production Deployment

### Using Docker

```dockerfile
# Don't copy .env file
# Use environment variables or secrets

ENV DATABASE_URL=postgresql://user:pass@db:5432/stellar_insights
ENV REDIS_URL=redis://redis:6379
# ... other variables
```

### Using Kubernetes

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: stellar-insights-secrets
type: Opaque
stringData:
  database-url: postgresql://user:pass@db:5432/stellar_insights
  redis-url: redis://redis:6379
---
apiVersion: apps/v1
kind: Deployment
spec:
  template:
    spec:
      containers:
      - name: backend
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: stellar-insights-secrets
              key: database-url
```

### Using systemd

```ini
[Service]
Environment="DATABASE_URL=postgresql://user:pass@localhost:5432/stellar_insights"
Environment="REDIS_URL=redis://localhost:6379"
EnvironmentFile=/etc/stellar-insights/backend.env
```

## Security Checklist

- [ ] `.env` is in `.gitignore`
- [ ] `.env` file permissions are 600
- [ ] Strong passwords used for all credentials
- [ ] API keys are kept secret
- [ ] Production uses secrets management
- [ ] CORS is properly configured (not "*")
- [ ] Database credentials are rotated regularly
- [ ] Logs don't contain sensitive data
- [ ] `.env.example` has no real secrets

## Additional Resources

- [DATABASE_POOL_CONFIG.md](./DATABASE_POOL_CONFIG.md) - Pool configuration details
- [SQLx Documentation](https://docs.rs/sqlx/) - Database driver
- [dotenv Documentation](https://docs.rs/dotenv/) - Environment loading
