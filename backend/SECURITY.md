# Security Guidelines

This document outlines security best practices for the Stellar Insights backend.

## Environment Variables & Secrets Management

### ✅ What We Do Right

1. **`.env` is in `.gitignore`**
   - Never committed to version control
   - Separate `.env.example` template provided

2. **Credential Sanitization**
   - Database URLs sanitized in logs
   - API keys redacted in output
   - Passwords never logged

3. **Startup Validation**
   - Required variables checked on startup
   - Format validation for ports and numbers
   - Fail-fast with clear error messages

4. **Secure Defaults**
   - Localhost binding by default
   - Reasonable connection limits
   - CORS restricted (not "*")

### 🔒 Security Checklist

Before deploying to production, verify:

- [ ] `.env` file is NOT in version control
- [ ] `.env` file permissions are `600` (owner read/write only)
- [ ] Strong passwords used (16+ characters, mixed case, numbers, symbols)
- [ ] API keys are kept secret and rotated regularly
- [ ] Database credentials are unique per environment
- [ ] CORS is properly configured (specific origins, not "*")
- [ ] Redis authentication is enabled in production
- [ ] TLS/SSL enabled for database connections
- [ ] Logs don't contain sensitive data
- [ ] `.env.example` contains no real secrets

### 🚨 Common Security Mistakes to Avoid

1. **DON'T commit `.env` files**
   ```bash
   # Bad - never do this!
   git add .env
   git commit -m "Add config"
   ```

2. **DON'T use weak passwords**
   ```bash
   # Bad
   DATABASE_URL=postgresql://admin:password@localhost/db
   
   # Good
   DATABASE_URL=postgresql://admin:Kx9$mP2#vL8@qR5&nW4!@localhost/db
   ```

3. **DON'T use wildcard CORS in production**
   ```bash
   # Bad
   CORS_ALLOWED_ORIGINS=*
   
   # Good
   CORS_ALLOWED_ORIGINS=https://stellar-insights.com
   ```

4. **DON'T log sensitive data**
   ```rust
   // Bad
   tracing::info!("Database password: {}", password);
   
   // Good
   tracing::info!("Database connected successfully");
   ```

5. **DON'T hardcode secrets**
   ```rust
   // Bad
   let api_key = "sk_live_abc123";
   
   // Good
   let api_key = env::var("API_KEY")?;
   ```

## Database Security

### Connection Security

1. **Use TLS for database connections**
   ```bash
   DATABASE_URL=postgresql://user:pass@host/db?sslmode=require
   ```

2. **Limit connection pool size**
   ```bash
   DB_POOL_MAX_CONNECTIONS=20  # Prevent resource exhaustion
   ```

3. **Set connection timeouts**
   ```bash
   DB_POOL_CONNECT_TIMEOUT_SECONDS=30  # Prevent hanging connections
   ```

### Access Control

1. **Use least privilege principle**
   - Application user should only have necessary permissions
   - No `SUPERUSER` or `CREATEDB` privileges needed

2. **Separate credentials per environment**
   - Development, staging, and production use different credentials
   - Never reuse production credentials

3. **Regular credential rotation**
   - Rotate database passwords quarterly
   - Update application configuration accordingly

## API Security

### Rate Limiting

Rate limiting is configured per endpoint:

```rust
rate_limiter.register_endpoint(
    "/api/anchors".to_string(),
    RateLimitConfig {
        requests_per_minute: 100,
        whitelist_ips: vec![],
    },
).await;
```

### CORS Configuration

Restrict origins to known domains:

```bash
# Development
CORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:3001

# Production
CORS_ALLOWED_ORIGINS=https://stellar-insights.com,https://www.stellar-insights.com
```

### Authentication

SEP-10 authentication is implemented for protected endpoints:

- Challenge/response authentication
- JWT token validation
- Refresh token support

## Production Deployment

### Secrets Management

Use a secrets management service:

**AWS Secrets Manager:**
```bash
aws secretsmanager get-secret-value \
  --secret-id stellar-insights/database-url \
  --query SecretString --output text
```

**HashiCorp Vault:**
```bash
vault kv get -field=database_url secret/stellar-insights
```

**Kubernetes Secrets:**
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: stellar-insights-secrets
type: Opaque
stringData:
  database-url: postgresql://user:pass@host/db
```

### Environment Variables in Production

**Docker:**
```dockerfile
# Use build args for non-sensitive config
ARG SERVER_PORT=8080
ENV SERVER_PORT=${SERVER_PORT}

# Use secrets for sensitive data
# Pass via docker run -e or docker-compose
```

**systemd:**
```ini
[Service]
# Load from secure file
EnvironmentFile=/etc/stellar-insights/secrets.env
# File should be owned by service user with 600 permissions
```

**Kubernetes:**
```yaml
env:
- name: DATABASE_URL
  valueFrom:
    secretKeyRef:
      name: stellar-insights-secrets
      key: database-url
```

### File Permissions

```bash
# Secure .env file
chmod 600 .env
chown app-user:app-group .env

# Secure application directory
chmod 750 /opt/stellar-insights
chown -R app-user:app-group /opt/stellar-insights
```

## Monitoring & Auditing

### Security Logging

The application logs security-relevant events:

- Authentication attempts
- Failed validations
- Rate limit violations
- Configuration errors

### Audit Trail

Monitor these logs for security issues:

```bash
# Failed authentication
grep "Authentication failed" /var/log/stellar-insights/app.log

# Rate limit violations
grep "Rate limit exceeded" /var/log/stellar-insights/app.log

# Configuration errors
grep "Environment configuration errors" /var/log/stellar-insights/app.log
```

### Metrics to Monitor

- Failed authentication rate
- Rate limit hit rate
- Database connection errors
- Unusual traffic patterns

## Incident Response

### If Credentials Are Compromised

1. **Immediately rotate credentials**
   ```bash
   # Update database password
   ALTER USER stellar_insights WITH PASSWORD 'new_secure_password';
   
   # Update .env file
   # Restart application
   ```

2. **Review access logs**
   - Check for unauthorized access
   - Identify affected resources

3. **Notify stakeholders**
   - Security team
   - DevOps team
   - Management

4. **Update documentation**
   - Document incident
   - Update procedures

### If `.env` Is Committed

1. **Remove from git history**
   ```bash
   # Remove file from git
   git rm --cached backend/.env
   git commit -m "Remove .env from version control"
   
   # Remove from history (use with caution!)
   git filter-branch --force --index-filter \
     "git rm --cached --ignore-unmatch backend/.env" \
     --prune-empty --tag-name-filter cat -- --all
   
   # Force push (coordinate with team!)
   git push origin --force --all
   ```

2. **Rotate ALL credentials**
   - Database passwords
   - API keys
   - Redis passwords
   - Any other secrets in the file

3. **Review commit history**
   - Check if file was in previous commits
   - Verify complete removal

## Security Updates

### Dependency Management

1. **Regular updates**
   ```bash
   cargo update
   cargo audit
   ```

2. **Security advisories**
   - Monitor RustSec Advisory Database
   - Subscribe to security mailing lists

3. **Automated scanning**
   - Use Dependabot or similar
   - CI/CD security checks

## Contact

For security issues, please email: security@stellar-insights.com

**DO NOT** open public GitHub issues for security vulnerabilities.

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [Stellar Security Best Practices](https://developers.stellar.org/docs/building-apps/security)
- [12-Factor App](https://12factor.net/config)
