# IP Whitelisting for Admin Endpoints

## Overview

This implementation provides IP-based access control for admin endpoints in the Stellar Insights backend. It restricts access to sensitive administrative routes to a configurable list of trusted IP addresses and CIDR ranges.

## Features

- ✅ Single IP address whitelisting (IPv4 and IPv6)
- ✅ CIDR range support (e.g., `192.168.1.0/24`)
- ✅ Multiple network support
- ✅ Proxy/load balancer support (`X-Forwarded-For` and `X-Real-IP` headers)
- ✅ Configurable via environment variables
- ✅ Secure logging (blocked attempts logged without exposing sensitive info)
- ✅ Proper HTTP 403 responses for blocked requests
- ✅ Edge case handling (malformed IPs, header injection prevention)
- ✅ Comprehensive test coverage

## Protected Endpoints

The following admin endpoints are protected by IP whitelisting:

- `GET /api/admin/analytics/overview` - API usage analytics
- `GET /api/cache/stats` - Cache statistics
- `POST /api/cache/reset` - Reset cache statistics
- `GET /api/db/pool-metrics` - Database connection pool metrics

## Configuration

### Environment Variables

Add the following to your `.env` file:

```bash
# Required: Comma-separated list of allowed IPs and CIDR ranges
ADMIN_IP_WHITELIST=127.0.0.1,::1

# Optional: Trust X-Forwarded-For header (default: false)
# Set to true when behind a reverse proxy or load balancer
ADMIN_IP_TRUST_PROXY=false

# Optional: Maximum IPs to check in X-Forwarded-For chain (default: 3)
# Prevents header injection attacks
ADMIN_IP_MAX_FORWARDED=3
```

### Configuration Examples

#### Development (localhost only)
```bash
ADMIN_IP_WHITELIST=127.0.0.1,::1
ADMIN_IP_TRUST_PROXY=false
```

#### Production (specific IPs)
```bash
ADMIN_IP_WHITELIST=203.0.113.50,198.51.100.100
ADMIN_IP_TRUST_PROXY=true
```

#### Production (office network)
```bash
ADMIN_IP_WHITELIST=203.0.113.0/24
ADMIN_IP_TRUST_PROXY=true
```

#### Production (multiple networks)
```bash
ADMIN_IP_WHITELIST=203.0.113.0/24,198.51.100.50,2001:db8::/32
ADMIN_IP_TRUST_PROXY=true
```

## Proxy/Load Balancer Support

### When to Enable `ADMIN_IP_TRUST_PROXY`

Enable this setting when your application is behind:
- Nginx reverse proxy
- AWS Application Load Balancer (ALB)
- AWS Network Load Balancer (NLB)
- Cloudflare
- Any other reverse proxy or CDN

### How It Works

When `ADMIN_IP_TRUST_PROXY=true`:
1. The middleware checks the `X-Forwarded-For` header first
2. Falls back to `X-Real-IP` header if `X-Forwarded-For` is not present
3. Falls back to direct connection IP if neither header is present

The leftmost IP in `X-Forwarded-For` is considered the original client IP.

### Security Considerations

**⚠️ Important:** Only enable `ADMIN_IP_TRUST_PROXY` if you trust your proxy/load balancer!

If enabled without a trusted proxy, attackers can spoof the `X-Forwarded-For` header to bypass IP restrictions.

**Protection against header injection:**
- The `ADMIN_IP_MAX_FORWARDED` setting limits how many IPs are checked in the chain
- Default is 3, which is sufficient for most setups
- This prevents attackers from injecting long chains of IPs

## IP Address Formats

### IPv4 Examples

```bash
# Single IP
ADMIN_IP_WHITELIST=192.168.1.100

# CIDR range
ADMIN_IP_WHITELIST=192.168.1.0/24

# Multiple IPs
ADMIN_IP_WHITELIST=192.168.1.100,10.0.0.1,172.16.0.1
```

### IPv6 Examples

```bash
# Single IPv6
ADMIN_IP_WHITELIST=::1

# IPv6 CIDR range
ADMIN_IP_WHITELIST=2001:db8::/32

# Mixed IPv4 and IPv6
ADMIN_IP_WHITELIST=192.168.1.0/24,::1,2001:db8::/32
```

## Error Responses

### Blocked Access (403 Forbidden)

```json
{
  "error": "Access denied: IP address not whitelisted"
}
```

### Invalid IP Configuration (403 Forbidden)

```json
{
  "error": "Access denied: Unable to verify IP address"
}
```

## Logging

### Successful Access

```
DEBUG client_ip=192.168.1.100 path=/api/admin/analytics/overview "IP whitelist: allowed access to admin endpoint"
```

### Blocked Access

```
WARN client_ip=203.0.113.99 path=/api/admin/analytics/overview method=GET "IP whitelist: blocked access attempt to admin endpoint"
```

**Note:** Logs do not expose sensitive information beyond the client IP and endpoint path.

## Testing

### Running Tests

```bash
cd backend
cargo test ip_whitelist
```

### Test Coverage

The test suite includes:
- ✅ Single IP whitelisting (IPv4 and IPv6)
- ✅ CIDR range matching
- ✅ Multiple network configurations
- ✅ X-Forwarded-For header handling
- ✅ X-Real-IP header handling
- ✅ Proxy trust settings
- ✅ Malformed IP handling
- ✅ Header injection prevention
- ✅ Edge cases (empty whitelist, invalid formats)

### Manual Testing

#### Test with curl (direct connection)

```bash
# Should succeed if 127.0.0.1 is whitelisted
curl http://localhost:8080/api/admin/analytics/overview

# Should fail (403) if not whitelisted
curl http://localhost:8080/api/admin/analytics/overview
```

#### Test with curl (behind proxy)

```bash
# Simulate X-Forwarded-For header
curl -H "X-Forwarded-For: 203.0.113.50" http://localhost:8080/api/admin/analytics/overview
```

## Deployment Checklist

- [ ] Set `ADMIN_IP_WHITELIST` with production IPs/ranges
- [ ] Set `ADMIN_IP_TRUST_PROXY=true` if behind proxy/load balancer
- [ ] Verify proxy correctly sets `X-Forwarded-For` or `X-Real-IP` headers
- [ ] Test access from whitelisted IPs
- [ ] Test access from non-whitelisted IPs (should be blocked)
- [ ] Monitor logs for blocked access attempts
- [ ] Document whitelisted IPs in your infrastructure documentation

## Troubleshooting

### Issue: All requests are blocked

**Solution:**
1. Check that `ADMIN_IP_WHITELIST` is set correctly
2. Verify the IP format (use `192.168.1.1` not `192.168.1.1/32` for single IPs, though both work)
3. Check application logs for configuration errors

### Issue: Requests blocked when behind proxy

**Solution:**
1. Set `ADMIN_IP_TRUST_PROXY=true`
2. Verify your proxy is setting `X-Forwarded-For` or `X-Real-IP` headers
3. Check the leftmost IP in `X-Forwarded-For` matches your whitelist

### Issue: Cannot determine client IP

**Solution:**
1. Ensure your proxy is configured to pass client IP information
2. For nginx: `proxy_set_header X-Real-IP $remote_addr;`
3. For nginx: `proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;`

## Security Best Practices

1. **Principle of Least Privilege**: Only whitelist IPs that absolutely need admin access
2. **Regular Audits**: Review whitelisted IPs regularly and remove unused entries
3. **Monitor Logs**: Set up alerts for repeated blocked access attempts
4. **Use CIDR Ranges Carefully**: Prefer specific IPs over broad ranges when possible
5. **Combine with Authentication**: IP whitelisting is a defense-in-depth measure, not a replacement for authentication
6. **Document Changes**: Keep a record of why each IP is whitelisted

## Performance Impact

The IP whitelist middleware has minimal performance impact:
- O(n) complexity where n is the number of whitelisted networks
- Typically < 1ms overhead per request
- No database queries or external API calls
- Efficient CIDR matching using the `ipnetwork` crate

## Future Enhancements

Potential improvements for future versions:
- Dynamic whitelist updates without restart
- Rate limiting per IP
- Temporary IP bans for repeated violations
- Integration with external IP reputation services
- Admin UI for whitelist management

## References

- [RFC 7239 - Forwarded HTTP Extension](https://tools.ietf.org/html/rfc7239)
- [OWASP - IP Whitelist](https://cheatsheetseries.owasp.org/cheatsheets/Access_Control_Cheat_Sheet.html)
- [ipnetwork crate documentation](https://docs.rs/ipnetwork/)
