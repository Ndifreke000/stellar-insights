---
title: "[SECURITY] Database Credentials Exposed in Version Control"
labels: security, critical, configuration, backend
assignees: ''
---

## 🔒 Security Issue

The `backend/.env` file containing database connection strings and potentially sensitive configuration is committed to version control. This is a security risk and violates best practices for secrets management.

## ⚠️ Current Behavior

- `.env` file is tracked in git
- Database URL visible in repository
- No secrets management strategy
- Risk of credential exposure
- Configuration details leaked

## ✅ Expected Behavior

- `.env` file in `.gitignore`
- `.env.example` template without secrets
- Documentation for local setup
- Use environment variables in production
- Consider secrets management tools

## 📁 Affected Files

- `backend/.env` (should be removed from git)
- `backend/.gitignore` (should include .env)
- `.env.example` (should be created)
- `backend/README.md` (setup documentation)

## 🚨 Security Risks

1. **Database credentials exposed** - Connection strings visible
2. **API keys potentially visible** - Third-party service keys
3. **Internal URLs/endpoints revealed** - Infrastructure details
4. **Configuration details leaked** - System architecture exposed

## 📋 Acceptance Criteria

- [ ] Remove `backend/.env` from git history
- [ ] Add `.env` to `.gitignore`
- [ ] Create `backend/.env.example` template
- [ ] Update README with setup instructions
- [ ] Document all environment variables
- [ ] Add validation for required env vars on startup
- [ ] Consider using `dotenv` crate properly
- [ ] Audit repository for other exposed secrets

## 🚀 Implementation Steps

1. **Create `.env.example` with placeholder values**
   ```bash
   DATABASE_URL=sqlite:./stellar_insights.db
   SERVER_HOST=127.0.0.1
   SERVER_PORT=8080
   RUST_LOG=info
   REDIS_URL=redis://127.0.0.1:6379
   RPC_MOCK_MODE=false
   STELLAR_RPC_URL=https://stellar.api.onfinality.io/public
   STELLAR_HORIZON_URL=https://horizon.stellar.org
   PRICE_FEED_API_KEY=your_api_key_here
   ```

2. **Add `.env` to `.gitignore`**
   ```gitignore
   # Environment variables
   .env
   .env.local
   .env.*.local
   ```

3. **Remove `.env` from git history**
   ```bash
   git rm --cached backend/.env
   git commit -m "Remove .env from version control [SECURITY]"
   git push
   ```

4. **Update README with environment setup section**

5. **Add startup validation for required variables**
   ```rust
   fn validate_env_vars() -> Result<()> {
       let required = vec![
           "DATABASE_URL",
           "SERVER_HOST",
           "SERVER_PORT",
           "REDIS_URL",
       ];
       
       for var in required {
           std::env::var(var)
               .map_err(|_| anyhow!("Missing required env var: {}", var))?;
       }
       
       Ok(())
   }
   ```

6. **Document all environment variables**

## 📝 Environment Variables to Document

```bash
# Database Configuration
DATABASE_URL=sqlite:./stellar_insights.db

# Server Configuration
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
RUST_LOG=info

# Redis Configuration
REDIS_URL=redis://127.0.0.1:6379

# Stellar RPC Configuration
RPC_MOCK_MODE=false
STELLAR_RPC_URL=https://stellar.api.onfinality.io/public
STELLAR_HORIZON_URL=https://horizon.stellar.org

# Price Feed Configuration (optional)
PRICE_FEED_PROVIDER=coingecko
PRICE_FEED_API_KEY=your_api_key_here
PRICE_FEED_CACHE_TTL_SECONDS=900

# CORS Configuration
CORS_ALLOWED_ORIGINS=http://localhost:3000

# Cache Configuration
CACHE_TTL_SECONDS=300
```

## 🔐 Additional Security Recommendations

1. **Use secrets management in production**
   - AWS Secrets Manager
   - HashiCorp Vault
   - Kubernetes Secrets

2. **Rotate exposed credentials**
   - Change database passwords
   - Regenerate API keys
   - Update service tokens

3. **Audit for other secrets**
   - Search for API keys in code
   - Check for hardcoded passwords
   - Review configuration files

4. **Add pre-commit hooks**
   - Prevent committing `.env` files
   - Scan for secrets before commit
   - Use tools like `git-secrets`

## 🏷️ Priority

**Critical** - Active security vulnerability

## 📚 References

- [OWASP: Sensitive Data Exposure](https://owasp.org/www-project-top-ten/2017/A3_2017-Sensitive_Data_Exposure)
- [12 Factor App: Config](https://12factor.net/config)
- [GitHub: Removing sensitive data](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository)
