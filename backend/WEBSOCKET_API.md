# WebSocket API Documentation

## Overview

The Stellar Insights WebSocket API provides real-time updates for corridor metrics, anchor status changes, and payment events. Clients can subscribe to specific channels to receive targeted updates.

## Connection

Connect to the WebSocket endpoint:
```
ws://localhost:8080/ws
```

Optional authentication via query parameter:
```
ws://localhost:8080/ws?token=your_auth_token
```

## Message Format

All messages are JSON objects with a `type` field indicating the message type.

## Client-to-Server Messages

### Subscribe to Channels
```json
{
  "type": "subscribe",
  "channels": ["corridor:USDC-XLM", "anchor:GXXX..."]
}
```

### Unsubscribe from Channels
```json
{
  "type": "unsubscribe",
  "channels": ["corridor:USDC-XLM"]
}
```

### Ping
```json
{
  "type": "ping",
  "timestamp": 1708425000
}
```

## Server-to-Client Messages

### Connection Established
```json
{
  "type": "connected",
  "connection_id": "uuid-string"
}
```

### Subscription Confirmation
```json
{
  "type": "subscription_confirm",
  "channels": ["corridor:USDC-XLM"],
  "status": "subscribed"
}
```

### Corridor Update
```json
{
  "type": "corridor_update",
  "corridor_key": "USDC-XLM",
  "asset_a_code": "USDC",
  "asset_a_issuer": "issuer_address",
  "asset_b_code": "XLM",
  "asset_b_issuer": "native",
  "success_rate": 94.5,
  "health_score": 92.0,
  "last_updated": "2026-02-20T10:30:00Z"
}
```

### Anchor Status Update
```json
{
  "type": "anchor_update",
  "anchor_id": "uuid-string",
  "name": "Anchor Name",
  "reliability_score": 95.2,
  "status": "active"
}
```

### New Payment Event
```json
{
  "type": "new_payment",
  "corridor_id": "USDC-XLM",
  "amount": 1000.50,
  "successful": true,
  "timestamp": "2026-02-20T10:30:00Z"
}
```

### Health Alert
```json
{
  "type": "health_alert",
  "corridor_id": "USDC-PHP",
  "severity": "warning",
  "message": "Success rate dropped below 85%",
  "timestamp": "2026-02-20T10:30:00Z"
}
```

### Connection Status
```json
{
  "type": "connection_status",
  "status": "connected"
}
```

### Pong Response
```json
{
  "type": "pong",
  "timestamp": 1708425000
}
```

### Error Message
```json
{
  "type": "error",
  "message": "Error description"
}
```

## Channel Naming Convention

- **Corridors**: `corridor:{corridor_key}` (e.g., `corridor:USDC-XLM`)
- **Anchors**: `anchor:{anchor_id}` (e.g., `anchor:uuid-string`)
- **Payments**: `payments:{corridor_key}` (e.g., `payments:USDC-XLM`)

## Update Frequencies

- **Corridor Metrics**: Every 30 seconds
- **Anchor Status**: Immediate on status change
- **Payment Events**: Real-time as they occur
- **Health Alerts**: Immediate when triggered

## Connection Management

### Automatic Reconnection
The client should implement exponential backoff for reconnection attempts:
- Initial delay: 1 second
- Maximum delay: 30 seconds
- Backoff multiplier: 1.5

### Heartbeat
The server sends ping messages every 30 seconds. Clients should respond with pong messages to maintain the connection.

### Error Handling
- Invalid JSON messages are ignored
- Unknown message types are logged but don't close the connection
- Authentication failures result in connection closure

## Rate Limiting

- Maximum 100 subscription requests per minute per connection
- Maximum 1000 messages per minute per connection
- Connections exceeding limits may be temporarily throttled

## Example Client Implementation (JavaScript)

```javascript
class StellarInsightsWebSocket {
  constructor(url) {
    this.url = url;
    this.ws = null;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;
    this.subscriptions = new Set();
  }

  connect() {
    this.ws = new WebSocket(this.url);
    
    this.ws.onopen = () => {
      console.log('Connected to Stellar Insights WebSocket');
      this.reconnectAttempts = 0;
      // Re-subscribe to previous channels
      if (this.subscriptions.size > 0) {
        this.subscribe([...this.subscriptions]);
      }
    };

    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      this.handleMessage(message);
    };

    this.ws.onclose = () => {
      console.log('WebSocket connection closed');
      this.attemptReconnect();
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };
  }

  subscribe(channels) {
    channels.forEach(channel => this.subscriptions.add(channel));
    this.send({
      type: 'subscribe',
      channels: channels
    });
  }

  unsubscribe(channels) {
    channels.forEach(channel => this.subscriptions.delete(channel));
    this.send({
      type: 'unsubscribe',
      channels: channels
    });
  }

  send(message) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  handleMessage(message) {
    switch (message.type) {
      case 'corridor_update':
        this.onCorridorUpdate(message);
        break;
      case 'health_alert':
        this.onHealthAlert(message);
        break;
      case 'ping':
        this.send({ type: 'pong', timestamp: message.timestamp });
        break;
      // Handle other message types...
    }
  }

  attemptReconnect() {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      const delay = Math.pow(1.5, this.reconnectAttempts) * 1000;
      setTimeout(() => {
        this.reconnectAttempts++;
        this.connect();
      }, delay);
    }
  }
}
```

## Security Considerations

- Use WSS (WebSocket Secure) in production
- Implement proper authentication and authorization
- Validate all incoming messages
- Rate limit connections and messages
- Monitor for suspicious activity

## Monitoring and Metrics

The WebSocket service exposes the following metrics:
- Active connection count
- Messages sent/received per second
- Subscription counts per channel
- Connection duration statistics
- Error rates and types