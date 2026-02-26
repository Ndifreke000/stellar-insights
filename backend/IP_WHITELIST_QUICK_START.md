# IP Whitelisting Quick Start Guide

## 🚀 Quick Setup (5 minutes)

### Step 1: Add to `.env`

```bash
# For local development
ADMIN_IP_WHITELIST=127.0.0.1,::1
ADMIN_IP_TRUST_PROXY=false
```

### Step 2: Restart the server

```bash
cargo run
```

### Step 3: Test it works

```bash
# Should work (if running locally)
curl http://localhost:8080/api/admin/analytics/overview

# Should return 403 Forbidden
curl -H "X-Forwarded-For: 1.2.3.4" http://localhost:8080/api/admin/analytics/overview
```

## 🏢 Production Setup

### Behind AWS ALB or Nginx

```bash
# Your office/VPN IP or range
ADMIN_IP_WHITELIST=203.0.113.0/24
ADMIN_IP_TRUST_PROXY=true
ADMIN_IP_MAX_FORWARDED=3
```

### Direct Connection (No Proxy)

```bash
# Specific admin IPs
ADMIN_IP_WHITELIST=203.0.113.50,198.51.100.100
ADMIN_IP_TRUST_PROXY=false
```

## 📋 Protected Endpoints

These endpoints now require whitelisted IPs:

- `GET /api/admin/analytics/overview`
- `GET /api/cache/stats`
- `POST /api/cache/reset`
- `GET /api/db/pool-metrics`

## ⚠️ Common Issues

### "All requests blocked"
- Check `ADMIN_IP_WHITELIST` is set
- Verify IP format: `192.168.1.1` or `192.168.1.0/24`

### "Blocked when behind proxy"
- Set `ADMIN_IP_TRUST_PROXY=true`
- Verify proxy sets `X-Forwarded-For` header

### "Cannot determine client IP"
- Check proxy configuration
- For nginx: `proxy_set_header X-Real-IP $remote_addr;`

## 🧪 Testing

```bash
# Run tests
cargo test ip_whitelist

# Check logs
tail -f logs/app.log | grep "IP whitelist"
```

## 📚 Full Documentation

See [IP_WHITELIST_DOCUMENTATION.md](./IP_WHITELIST_DOCUMENTATION.md) for complete details.
