# IP Whitelisting for Admin Endpoints

## Overview

Admin endpoints are restricted to a configurable IP allowlist for enhanced security. Only requests from whitelisted IPs can access admin endpoints.

## Fail-Closed Default

If the allowlist is empty or unconfigured, **all admin access is denied** (fail-closed). At least one IP must be added before any admin operations are possible.

## Configuration

### Database-Backed

IPs are stored in the `admin_ip_whitelist` table:
- `ip_or_cidr`: Single IP or CIDR range
- `description`: Optional description
- `added_by_user_id`: Admin who added it
- `added_at`: Timestamp

### API Endpoints

- `GET /admin/ip-whitelist` - List all entries
- `POST /admin/ip-whitelist` - Add IP/CIDR
- `DELETE /admin/ip-whitelist/:ip_or_cidr` - Remove entry
- `POST /admin/ip-whitelist/check` - Check if IP is whitelisted

## Lockout Avoidance

Before removing an IP or allowing a removal, verify:
1. Current request IP is whitelisted (prevent lockout)
2. Removing won't leave zero entries (maintain fail-closed state)
3. At least one entry always remains

## IP Formats Supported

- **Single IPv4**: `192.168.1.1`
- **IPv4 CIDR**: `192.168.0.0/16`
- **Single IPv6**: `2001:db8::1`
- **IPv6 CIDR**: `2001:db8::/32`

## X-Forwarded-For Handling

When behind a reverse proxy, the middleware extracts the client IP from `X-Forwarded-For` header, trusting only the configured proxy hops (prevents spoofing).

## Audit Logging

All whitelist modifications are logged to the audit log with:
- Action: `admin_ip_whitelist_add`, `admin_ip_whitelist_remove`
- IP address and user who made the change
- Event type: `ip_whitelist_*`
