# Stellar Insights TypeScript SDK

TypeScript client for the Stellar Insights API and realtime feed.

## Install

```bash
npm install @stellar-insights/sdk
```

## Usage

```ts
import { StellarInsightsClient } from '@stellar-insights/sdk';

const client = new StellarInsightsClient({ baseUrl: 'http://localhost:8080' });
const preflight = await client.preflight({
  sourceAsset: 'USDC',
  destinationAsset: 'NGN',
  amountUsd: 2500,
});
```

## Realtime WebSocket client

The SDK ships a typed, resumable WebSocket client. Channels are typed via the
`ChannelMap` interface, so subscribers receive typed payloads for each channel.

```ts
import { WebSocketManager } from '@stellar-insights/sdk';

const ws = new WebSocketManager({
  url: 'ws://localhost:8080/ws',
  // Optional: resume from a known sequence number.
  sinceSeq: 0,
});

// Typed subscription: `msg` is typed for the "corridors" channel.
const unsubscribe = ws.subscribe('corridors', (msg) => {
  console.log(msg.seq, msg.payload);
});

await ws.connect();
// ...
unsubscribe();
ws.close();
```

### Reconnection and resume

On an unexpected disconnect the client reconnects automatically using
exponential backoff with jitter. When reconnecting it sends the last observed
`seq` as `since_seq`, so the server can replay any missed messages. The client
tracks the highest `seq` seen across all channels.

- `reconnect: false` disables automatic reconnection.
- `maxRetries` caps the number of reconnect attempts (default `Infinity`).
- `baseDelayMs` / `maxDelayMs` tune the backoff window.

### Node and browser support

The client uses the global `WebSocket` implementation available in browsers and
in Node 22+. For older Node versions, pass a compatible constructor via the
`WebSocketImpl` option:

```ts
import WebSocket from 'ws';

const ws = new WebSocketManager({
  url: 'ws://localhost:8080/ws',
  WebSocketImpl: WebSocket as unknown as typeof globalThis.WebSocket,
});
```

## Development

```bash
npm install
npm test
```
