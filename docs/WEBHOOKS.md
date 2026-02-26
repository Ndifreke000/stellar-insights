# Webhook Support for Events

This document describes the webhook system implemented for Stellar Insights, which allows users to receive real-time notifications about corridor events, anchor status changes, and payment activities.

## Overview

The webhook system provides:
- Real-time event notifications via HTTP callbacks
- Support for multiple event types
- Event filtering capabilities
- Secure webhook delivery with HMAC signatures
- Retry logic for failed deliveries
- Comprehensive event logging

## Supported Event Types

### 1. Corridor Health Degradation (`corridor.health_degraded`)
Triggered when corridor metrics show significant degradation in performance.

**Payload:**
```json
{
  "corridor_key": "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->XLM:native",
  "old_metrics": {
    "success_rate": 0.95,
    "avg_latency_ms": 100.0,
    "p95_latency_ms": 150.0,
    "p99_latency_ms": 200.0,
    "liquidity_depth_usd": 1000000.0,
    "liquidity_volume_24h_usd": 500000.0,
    "total_attempts": 1000,
    "successful_payments": 950,
    "failed_payments": 50
  },
  "new_metrics": {
    "success_rate": 0.84,
    "avg_latency_ms": 180.0,
    "p95_latency_ms": 250.0,
    "p99_latency_ms": 350.0,
    "liquidity_depth_usd": 800000.0,
    "liquidity_volume_24h_usd": 400000.0,
    "total_attempts": 1000,
    "successful_payments": 840,
    "failed_payments": 160
  },
  "severity": "warning",
  "changes": ["success_rate_dropped: 95.0% -> 84.0%", "latency_increased: 150ms -> 250ms"]
}
```

### 2. Anchor Status Changed (`anchor.status_changed`)
Triggered when an anchor's reliability score or status changes significantly.

**Payload:**
```json
{
  "anchor_id": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
  "name": "Stellar Development Foundation",
  "old_status": "healthy",
  "new_status": "degraded",
  "reliability_score": 85.0,
  "failed_txn_count": 5
}
```

### 3. Payment Created (`payment.created`)
Triggered when a new payment is processed and recorded in the ledger.

**Payload:**
```json
{
  "payment_id": "abc123-123456",
  "source": "GABC...",
  "destination": "GXYZ...",
  "asset_code": "USDC",
  "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
  "amount": 100.50,
  "timestamp": "2023-01-01T12:00:00Z"
}
```

### 4. Corridor Liquidity Dropped (`corridor.liquidity_dropped`)
Triggered when corridor liquidity drops below a threshold.

**Payload:**
```json
{
  "corridor_key": "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN->XLM:native",
  "liquidity_depth_usd": 700000.0,
  "threshold": 1000000.0,
  "liquidity_trend": "decreasing",
  "severity": "warning"
}
```

## Webhook API Endpoints

### Register Webhook
```http
POST /api/webhooks
Content-Type: application/json
Authorization: Bearer <token>

{
  "url": "https://your-app.com/webhook",
  "event_types": ["corridor.health_degraded", "anchor.status_changed"],
  "filters": {
    "severity": "critical"
  }
}
```

### List Webhooks
```http
GET /api/webhooks
Authorization: Bearer <token>
```

### Delete Webhook
```http
DELETE /api/webhooks/{webhook_id}
Authorization: Bearer <token>
```

### Test Webhook
```http
POST /api/webhooks/{webhook_id}/test
Authorization: Bearer <token>
```

## Webhook Delivery

### Headers
All webhook deliveries include the following headers:

- `X-Zapier-Event`: The event type (e.g., `corridor.health_degraded`)
- `X-Zapier-Signature`: HMAC-SHA256 signature for verification
- `X-Zapier-Timestamp`: Unix timestamp of the delivery
- `X-Zapier-Delivery-ID`: Unique delivery ID for idempotency
- `Content-Type`: `application/json`

### Event Envelope
All events are wrapped in an envelope:

```json
{
  "id": "delivery-uuid",
  "event": "corridor.health_degraded",
  "timestamp": 1672574400,
  "data": {
    // Event-specific payload
  }
}
```

### Signature Verification
To verify webhook authenticity:

```python
import hmac
import hashlib

def verify_signature(payload, secret, signature):
    expected = hmac.new(
        secret.encode('utf-8'),
        payload.encode('utf-8'),
        hashlib.sha256
    ).hexdigest()
    
    expected_signature = f"sha256={expected}"
    return hmac.compare_digest(signature, expected_signature)
```

## Event Filtering

Webhooks can be configured with filters to receive only relevant events:

```json
{
  "url": "https://your-app.com/webhook",
  "event_types": ["corridor.health_degraded"],
  "filters": {
    "severity": "critical",
    "corridor_key": "USDC:*"
  }
}
```

### Supported Filter Operations
- **Exact match**: `{"severity": "critical"}`
- **Nested object matching**: `{"new_metrics.success_rate": 0.8}`

## Retry Logic

The webhook system implements automatic retry logic:

- **Maximum retries**: 3 attempts
- **Retry intervals**: Exponential backoff
- **Failure tracking**: All failures are logged with error details

## Security Features

### 1. HMAC Signatures
All webhooks are signed with HMAC-SHA256 using a per-webhook secret.

### 2. URL Validation
- Only HTTP/HTTPS URLs are allowed
- Private/internal IP addresses are blocked
- Cloud metadata endpoints are blocked
- Localhost addresses are blocked in production

### 3. Secret Encryption
Webhook secrets are encrypted at rest using AES-256-GCM.

## Monitoring and Logging

### Delivery Status
Webhook deliveries are tracked with the following statuses:
- `pending`: Waiting to be delivered
- `delivered`: Successfully delivered
- `failed`: Max retries exceeded

### Logging
All webhook activities are logged:
- Event triggers
- Delivery attempts
- Success/failure status
- Error details

## Configuration

### Environment Variables
- `ENCRYPTION_KEY`: 32-byte hex key for secret encryption
- `WEBHOOK_TIMEOUT`: HTTP request timeout (default: 10s)
- `WEBHOOK_RETRY_INTERVAL`: Base retry interval (default: 5s)

### Database Schema
The webhook system uses two main tables:
- `webhooks`: Stores webhook configurations
- `webhook_events`: Tracks delivery attempts and status

## Integration Examples

### Node.js Express
```javascript
const express = require('express');
const crypto = require('crypto');

const app = express();
app.use(express.raw({ type: 'application/json' }));

app.post('/webhook', (req, res) => {
  const signature = req.headers['x-zapier-signature'];
  const secret = 'your-webhook-secret';
  
  const expected = crypto
    .createHmac('sha256', secret)
    .update(req.body)
    .digest('hex');
  
  if (signature !== `sha256=${expected}`) {
    return res.status(401).send('Invalid signature');
  }
  
  const event = JSON.parse(req.body);
  console.log('Received event:', event);
  
  res.status(200).send('OK');
});
```

### Python Flask
```python
from flask import Flask, request, jsonify
import hmac
import hashlib

app = Flask(__name__)

@app.route('/webhook', methods=['POST'])
def webhook():
    signature = request.headers.get('X-Zapier-Signature')
    secret = 'your-webhook-secret'
    
    expected = hmac.new(
        secret.encode('utf-8'),
        request.data,
        hashlib.sha256
    ).hexdigest()
    
    if signature != f'sha256={expected}':
        return 'Invalid signature', 401
    
    event = request.get_json()
    print(f'Received event: {event}')
    
    return 'OK', 200
```

## Best Practices

1. **Always verify signatures** to ensure webhook authenticity
2. **Process events asynchronously** to avoid timeouts
3. **Implement idempotency** using the delivery ID
4. **Monitor delivery failures** and update webhook URLs if needed
5. **Use specific filters** to reduce unnecessary webhook traffic
6. **Handle network errors gracefully** with appropriate retry logic

## Troubleshooting

### Common Issues

1. **Signature verification failures**
   - Check that you're using the correct secret
   - Ensure you're verifying the raw request body
   - Verify the signature format (`sha256=...`)

2. **Missing events**
   - Check webhook event type subscriptions
   - Verify filter conditions match event data
   - Check webhook is active (`is_active = true`)

3. **Delivery failures**
   - Verify webhook URL is accessible
   - Check for network/firewall issues
   - Monitor webhook delivery logs

### Debugging Tools

- Use the `/api/webhooks/{id}/test` endpoint to test delivery
- Check webhook event logs for delivery status
- Monitor application logs for webhook errors

## Future Enhancements

Planned improvements to the webhook system:

1. **Advanced filtering**: Support for regex and complex conditions
2. **Batch delivery**: Group multiple events in single delivery
3. **Webhook statistics**: Detailed analytics and metrics
4. **Custom retry policies**: Per-webhook retry configurations
5. **Event replay**: Replay missed events during downtime
6. **Webhook templates**: Pre-configured webhook setups
