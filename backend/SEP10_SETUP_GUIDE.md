# 🚀 SEP-10 Setup Guide

Quick guide to configure SEP-10 authentication for Stellar Insights Backend.

---

## ⚡ Quick Start

### 1. Generate Stellar Keypair

**For Testnet (Development)**:
```bash
stellar keys generate --network testnet
```

**For Mainnet (Production)**:
```bash
stellar keys generate --network mainnet
```

**Output Example**:
```
Secret key: SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
Public key: GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H
```

⚠️ **IMPORTANT**: 
- Save the **secret key** securely (use a password manager or secrets vault)
- Use the **public key** for `SEP10_SERVER_PUBLIC_KEY`
- Never commit keys to version control

---

### 2. Configure Environment Variables

Create or update your `.env` file:

```bash
# Required: SEP-10 server public key
SEP10_SERVER_PUBLIC_KEY=GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H

# Optional: Home domain (defaults to stellar-insights.local)
SEP10_HOME_DOMAIN=localhost

# Required: Network passphrase (must match your Stellar network)
STELLAR_NETWORK_PASSPHRASE=Test SDF Network ; September 2015
```

**Network Passphrases**:
- **Testnet**: `Test SDF Network ; September 2015`
- **Mainnet**: `Public Global Stellar Network ; September 2015`

---

### 3. Start the Server

```bash
cd backend
cargo run
```

**Expected Output**:
```
INFO stellar_insights_backend: Starting Stellar Insights Backend
INFO stellar_insights_backend: Environment configuration:
INFO stellar_insights_backend:   SEP10_SERVER_PUBLIC_KEY: GBRPYHIL...
INFO stellar_insights_backend: SEP-10 authentication enabled with server key: GBRPYHIL...
INFO stellar_insights_backend: SEP-10 service initialized successfully
INFO stellar_insights_backend: Server starting on 127.0.0.1:8080
```

---

## ❌ Common Errors

### Error 1: Missing SEP10_SERVER_PUBLIC_KEY

```
Error: Environment configuration errors:
  - Missing required environment variable: SEP10_SERVER_PUBLIC_KEY
```

**Solution**: Set the environment variable in your `.env` file.

---

### Error 2: Placeholder Value

```
Error: Invalid value for environment variable SEP10_SERVER_PUBLIC_KEY: 'GXXXXXX...'
```

**Solution**: Replace the placeholder with a real Stellar public key (see step 1).

---

### Error 3: Invalid Format

```
Error: Invalid value for environment variable SEP10_SERVER_PUBLIC_KEY: 'GBRPYHIL...'
```

**Possible Causes**:
- Key doesn't start with 'G'
- Key is not exactly 56 characters
- Key contains invalid characters

**Solution**: Generate a new keypair using `stellar keys generate`.

---

## 🔒 Security Best Practices

### Development
- ✅ Use testnet keys
- ✅ Store keys in `.env` (not committed)
- ✅ Use different keys per developer
- ✅ Rotate keys periodically

### Production
- ✅ Use mainnet keys
- ✅ Store keys in secrets manager (AWS Secrets Manager, HashiCorp Vault, etc.)
- ✅ Use different keys per environment (staging, production)
- ✅ Implement key rotation policy
- ✅ Monitor authentication logs
- ✅ Set up alerts for failed auth attempts

---

## 🧪 Testing SEP-10 Authentication

### 1. Request Challenge

```bash
curl -X POST http://localhost:8080/api/auth/sep10/challenge \
  -H "Content-Type: application/json" \
  -d '{
    "account": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "home_domain": "localhost"
  }'
```

**Response**:
```json
{
  "transaction": "AAAAAgAAAAA...",
  "network_passphrase": "Test SDF Network ; September 2015"
}
```

### 2. Sign Challenge

Use Stellar SDK to sign the challenge transaction with your account's secret key.

### 3. Verify Challenge

```bash
curl -X POST http://localhost:8080/api/auth/sep10/verify \
  -H "Content-Type: application/json" \
  -d '{
    "transaction": "AAAAAgAAAAA...",
    "account": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
  }'
```

**Response**:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": 1708704000
}
```

### 4. Use Token

```bash
curl http://localhost:8080/api/protected-endpoint \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

---

## 📋 Environment Variables Reference

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SEP10_SERVER_PUBLIC_KEY` | ✅ Yes | None | Stellar public key for signing challenges |
| `SEP10_HOME_DOMAIN` | ❌ No | `stellar-insights.local` | Home domain for SEP-10 auth |
| `STELLAR_NETWORK_PASSPHRASE` | ✅ Yes | None | Network passphrase (testnet/mainnet) |

---

## 🔧 Troubleshooting

### Server Won't Start

1. Check environment variables are set:
   ```bash
   echo $SEP10_SERVER_PUBLIC_KEY
   ```

2. Verify `.env` file exists and is loaded:
   ```bash
   cat backend/.env
   ```

3. Check logs for specific error messages:
   ```bash
   RUST_LOG=debug cargo run
   ```

### Authentication Fails

1. Verify network passphrase matches:
   - Client and server must use same passphrase
   - Testnet vs Mainnet mismatch is common

2. Check home domain:
   - Must match between client and server
   - Case-sensitive

3. Verify challenge hasn't expired:
   - Challenges expire after 5 minutes
   - Request new challenge if expired

---

## 📚 Additional Resources

- [SEP-10 Specification](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md)
- [Stellar CLI Documentation](https://developers.stellar.org/docs/tools/developer-tools)
- [Security Fix Documentation](./SECURITY_FIX_SEP10.md)

---

## 💡 Tips

- **Use stellar CLI**: Install with `cargo install --locked stellar-cli`
- **Test locally first**: Always test on testnet before mainnet
- **Monitor logs**: Enable debug logging during development
- **Rotate keys**: Implement key rotation for production
- **Backup keys**: Store secret keys securely with backups

---

**Need Help?** Check the [Security Fix Documentation](./SECURITY_FIX_SEP10.md) for detailed information.
