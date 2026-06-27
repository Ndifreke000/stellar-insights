export type EventHandler = (event: unknown) => void;

interface ConnectionKey {
  rpcUrl: string;
  network: string;
}

interface ManagedConnection {
  ws: WebSocket;
  refCount: number;
  handlers: Set<EventHandler>;
}

function connectionKey(key: ConnectionKey): string {
  return `${key.rpcUrl}|${key.network}`;
}

const connections = new Map<string, ManagedConnection>();

export function acquireConnection(
  key: ConnectionKey,
  handler: EventHandler,
): void {
  const k = connectionKey(key);
  const existing = connections.get(k);

  if (existing) {
    existing.refCount++;
    existing.handlers.add(handler);
    return;
  }

  const wsUrl = key.rpcUrl.replace(/^http/, "ws");
  const ws = new WebSocket(wsUrl);

  const conn: ManagedConnection = {
    ws,
    refCount: 1,
    handlers: new Set([handler]),
  };

  ws.onmessage = (event) => {
    let parsed: unknown;
    try {
      parsed = JSON.parse(String(event.data));
    } catch {
      return;
    }
    for (const h of conn.handlers) {
      h(parsed);
    }
  };

  ws.onclose = () => {
    connections.delete(k);
  };

  connections.set(k, conn);
}

export function releaseConnection(
  key: ConnectionKey,
  handler: EventHandler,
): void {
  const k = connectionKey(key);
  const conn = connections.get(k);
  if (!conn) return;

  conn.handlers.delete(handler);
  conn.refCount--;

  if (conn.refCount <= 0) {
    conn.ws.close();
    connections.delete(k);
  }
}

export function closeAllConnections(): void {
  for (const conn of connections.values()) {
    conn.ws.close();
  }
  connections.clear();
}
