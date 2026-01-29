## WebSocket Real-time Updates

This implementation provides real-time updates from the Stellar Insights backend via WebSocket connections.

### Backend Implementation

#### WebSocket Endpoint
- **URL**: `ws://localhost:8080/ws` (development) or `wss://your-domain/ws` (production)
- **Authentication**: Optional token-based authentication via query parameter `?token=YOUR_TOKEN`

#### Message Types

The WebSocket server sends the following message types:

1. **`snapshot_update`** - New snapshot available
   ```json
   {
     "type": "snapshot_update",
     "snapshot_id": "uuid",
     "epoch": 123,
     "timestamp": "2024-01-01T00:00:00Z",
     "hash": "abc123..."
   }
   ```

2. **`corridor_update`** - Corridor metrics updated
   ```json
   {
     "type": "corridor_update",
     "corridor_id": "uuid",
     "corridor_key": "USDC:issuer->EURC:issuer",
     "success_rate": 95.5,
     "volume_usd": 1000000.0,
     "total_transactions": 1000
   }
   ```

3. **`anchor_update`** - Anchor metrics updated
   ```json
   {
     "type": "anchor_update",
     "anchor_id": "uuid",
     "name": "Anchor Name",
     "reliability_score": 98.5,
     "status": "green"
   }
   ```

4. **`ping`** / **`pong`** - Heartbeat mechanism
   ```json
   {
     "type": "ping",
     "timestamp": 1234567890
   }
   ```

5. **`connected`** - Connection established
   ```json
   {
     "type": "connected",
     "connection_id": "uuid"
   }
   ```

6. **`error`** - Error message
   ```json
   {
     "type": "error",
     "message": "Error description"
   }
   ```

#### Connection Management

- **Heartbeat**: Server sends ping every 30 seconds, client automatically responds with pong
- **Disconnection Handling**: Automatic reconnection with exponential backoff (up to 5 attempts)
- **Authentication**: Set `WS_AUTH_TOKEN` environment variable on the backend to require authentication

### Frontend Implementation

#### Basic Usage

```tsx
import { useWebSocket } from '@/hooks/useWebSocket';

function MyComponent() {
  const { isConnected, lastMessage } = useWebSocket({
    onMessage: (message) => {
      console.log('Received:', message);
    },
  });

  return (
    <div>
      <p>Status: {isConnected ? 'Connected' : 'Disconnected'}</p>
      {lastMessage && (
        <pre>{JSON.stringify(lastMessage, null, 2)}</pre>
      )}
    </div>
  );
}
```

#### Listening to Specific Updates

```tsx
import { useSnapshotUpdates, useCorridorUpdates, useAnchorUpdates } from '@/hooks/useWebSocket';

function DashboardComponent() {
  // Listen for snapshot updates
  useSnapshotUpdates((update) => {
    console.log('New snapshot:', update.snapshot_id);
    // Refresh snapshot data
  });

  // Listen for corridor updates
  useCorridorUpdates((update) => {
    console.log('Corridor updated:', update.corridor_key);
    // Update corridor data in state
  });

  // Listen for anchor updates
  useAnchorUpdates((update) => {
    console.log('Anchor updated:', update.name);
    // Update anchor data in state
  });

  return <div>Dashboard with real-time updates</div>;
}
```

#### Manual Connection Control

```tsx
import { useWebSocket } from '@/hooks/useWebSocket';

function MyComponent() {
  const { isConnected, connect, disconnect, ping } = useWebSocket({
    autoConnect: false, // Don't auto-connect
  });

  return (
    <div>
      <button onClick={connect}>Connect</button>
      <button onClick={disconnect}>Disconnect</button>
      <button onClick={ping}>Send Ping</button>
      <p>Status: {isConnected ? 'Connected' : 'Disconnected'}</p>
    </div>
  );
}
```

### Environment Configuration

Create a `.env.local` file in the frontend directory:

```env
# WebSocket URL (optional, defaults to ws://localhost:8080/ws in development)
NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws

# WebSocket port (optional, defaults to 8080)
NEXT_PUBLIC_WS_PORT=8080

# WebSocket authentication token (optional)
NEXT_PUBLIC_WS_TOKEN=your-token-here
```

For the backend, create a `.env` file:

```env
# WebSocket authentication token (optional)
WS_AUTH_TOKEN=your-token-here
```

### Broadcasting Updates

#### From Backend Handlers

The backend automatically broadcasts updates when:
- Creating or updating anchors
- Creating or updating corridors
- Generating new snapshots

To manually broadcast an update from anywhere in the backend:

```rust
use crate::broadcast::{broadcast_anchor_update, broadcast_corridor_update, broadcast_snapshot_update};

// Broadcast anchor update
broadcast_anchor_update(&ws_state, &anchor);

// Broadcast corridor update
broadcast_corridor_update(&ws_state, &corridor);

// Broadcast snapshot update
broadcast_snapshot_update(&ws_state, &snapshot);
```

### Testing

#### Backend Tests

```bash
cd backend
cargo test websocket
```

#### Manual Testing

1. Start the backend:
   ```bash
   cd backend
   cargo run
   ```

2. Connect with a WebSocket client (e.g., `wscat`):
   ```bash
   npm install -g wscat
   wscat -c ws://localhost:8080/ws
   ```

3. You should receive a `connected` message and periodic `ping` messages.

4. Trigger updates by making API calls to create/update anchors or corridors:
   ```bash
   curl -X POST http://localhost:8080/api/anchors \
     -H "Content-Type: application/json" \
     -d '{"name": "Test Anchor", "stellar_account": "GABC..."}'
   ```

5. Observe the WebSocket client receiving `anchor_update` messages.

### Security Considerations

1. **Authentication**: Always use tokens in production environments
2. **Rate Limiting**: Consider implementing rate limiting for WebSocket connections
3. **TLS**: Always use WSS (WebSocket Secure) in production
4. **CORS**: Configure CORS appropriately for your production domain

### Troubleshooting

#### Connection Issues

- Verify the WebSocket URL is correct
- Check firewall/proxy settings
- Ensure the backend is running
- Check browser console for errors

#### Authentication Failures

- Verify the token is correct
- Check that `WS_AUTH_TOKEN` is set on the backend
- Ensure the token is passed in the URL query parameter

#### Reconnection Problems

- Check the `maxReconnectAttempts` setting
- Verify `autoReconnect` is enabled
- Look for rate limiting on the server

### Performance Considerations

- The WebSocket connection is a singleton - only one connection per app instance
- Messages are broadcast to all connected clients efficiently
- Consider implementing message filtering on the client for performance
- Use React's `useCallback` and `useMemo` to prevent unnecessary re-renders
