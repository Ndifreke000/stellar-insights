# Webhooks

Stellar Insights provides webhook capabilities for receiving real-time notifications about events in the system. This document describes the webhook features, API endpoints, and implementation details.

## Features

### Per-Endpoint FIFO Ordering

Webhooks are delivered in strict FIFO (first-in-first-out) order **per endpoint**. This means:

- Events for a single webhook are delivered in the order they were created
- Multiple webhooks can receive deliveries in parallel
- Each webhook has its own dedicated queue

This ordering guarantee is essential for state events where the sequence of updates matters.

### Auto-Disable on Sustained Failure

Webhooks are automatically disabled after a configurable number of consecutive failures:

- **Default**: 5 consecutive failures
- **Configurable**: Set `max_consecutive_failures` during webhook creation
- **Notification**: Owner is notified via email/webhook when disabled
- **Re-enable**: Use the `/enable` endpoint to re-enable a disabled webhook

### Versioned Payloads

All webhook payloads include an `event_version` field:

- **Current version**: `v1`
- **Future-proof**: New versions will be added with backwards compatibility
- **Signature verification**: Version is included in the signature calculation

### Rotatable Secrets

Webhook secrets support rotation with a grace period:

- **Primary secret**: Used for signing new webhooks
- **Secondary secret**: Allows for rotation with overlap period
- **Dual verification**: Both signatures are valid during rotation
- **Rotation period**: Configurable (default 30 days)

### Delivery Logs API

Complete audit trail of webhook delivery attempts:

- **Status**: pending, delivered, failed
- **HTTP status codes**: Captured for debugging
- **Response bodies**: Stored for inspection
- **Error messages**: Captured for troubleshooting
- **Pagination**: Support for large history

## API Endpoints

### Register Webhook

```http
POST /api/v1/webhooks
Content-Type: application/json

{
  "url": "https://example.com/webhook",
  "event_types": ["payment.created", "corridor.health_degraded"],
  "filters": {"severity": "critical"},
  "event_version": "v1",
  "secret_rotation_enabled": true,
  "secret_rotation_period_days": 30
}
```

### List Webhooks

```http
GET /api/v1/webhooks
```

### Get Webhook Details

```http
GET /api/v1/webhooks/{id}
```

### Test Webhook

```http
POST /api/v1/webhooks/{id}/test
```

### Get Delivery Logs

```http
GET /api/v1/webhooks/{id}/delivery-logs?limit=50&offset=0
```

### Enable Webhook

```http
POST /api/v1/webhooks/{id}/enable
```

### Rotate Secrets

```http
POST /api/v1/webhooks/{id}/rotate-secrets
```

### Delete Webhook

```http
DELETE /api/v1/webhooks/{id}
```

## Event Payload Format

```json
{
  "id": "delivery-uuid",
  "event": "event_type",
  "timestamp": 1234567890,
  "event_version": "v1",
  "data": { /* event-specific data */ }
}
```

## Signature Verification

All webhooks include an `X-Zapier-Signature` header for verification:

```
X-Zapier-Signature: sha256=<hmac-sha256>
X-Zapier-Timestamp: <unix-timestamp>
X-Zapier-Delivery-ID: <uuid>
X-Zapier-Event-Version: v1
```

### Verification Steps

1. Extract the signature from the `X-Zapier-Signature` header
2. Compute HMAC-SHA256 of the request body using the webhook secret
3. Compare the computed signature with the received signature
4. Check timestamp to prevent replay attacks (within acceptable window)

## Event Types

| Event Type | Description |
|------------|-------------|
| `payment.created` | A new payment has been created |
| `corridor.health_degraded` | A corridor's health metrics have degraded |
| `anchor.status_changed` | An anchor's status has changed |
| `corridor.liquidity_dropped` | A corridor's liquidity has dropped below threshold |

## Error Handling

### Auto-Disable

When a webhook fails consecutively up to `max_consecutive_failures`:

1. The webhook is marked as inactive
2. A notification is sent to the owner
3. The webhook can be re-enabled via the `/enable` endpoint

### Retry Policy

- **Initial delay**: 500ms
- **Backoff**: Exponential (500ms, 1000ms, 2000ms, 4000ms, ...)
- **Max retries**: 3 per event
- **Max failures**: 5 consecutive (configurable)

## Configuration

Environment variables for webhook behavior:

| Variable | Default | Description |
|----------|---------|-------------|
| `ENCRYPTION_KEY` | (required) | Key for encrypting webhook secrets |
| `WEBHOOK_DISPATCHER_INTERVAL` | 5 | Seconds between dispatcher runs |
| `WEBHOOK_PARALLELISM` | 4 | Number of parallel dispatchers |
| `WEBHOOK_TIMEOUT` | 10 | Timeout in seconds for webhook requests |

## Security Considerations

1. **Secret Rotation**: Rotate secrets periodically using the `/rotate-secrets` endpoint
2. **Signature Verification**: Always verify webhook signatures on the receiver side
3. **HTTPS Only**: Configure webhooks to use HTTPS endpoints
4. **IP Whitelisting**: Consider using IP whitelisting on your webhook receiver
5. **Rate Limiting**: Webhooks are subject to rate limiting on the receiver side

## Monitoring

### Health Check

The webhook dispatcher exposes health metrics:

- `running`: Whether the dispatcher is active
- `consecutive_failures`: Number of consecutive failures
- `total_failures`: Total failures since startup

### Logs

Look for these log patterns:

```
Webhook delivered successfully: webhook_id=..., event=...
Webhook delivery failed (will retry): webhook_id=..., error=..., retries=...
Webhook delivery failed (max retries): webhook_id=..., error=...
Webhook auto-disabled: webhook_id=..., reason=...
```

## Migration Notes

For existing webhooks:

1. Run the database migration (`020_webhook_enhancements.sql`)
2. Existing webhooks will use default values:
   - `max_consecutive_failures`: 5
   - `secret_rotation_enabled`: false
   - `event_version`: "v1"
3. Test thoroughly before deploying to production

## Support

For issues or questions:

- Open an issue in the repository
- Check the CHANGELOG for recent changes
- Review the API documentation for specific endpoints