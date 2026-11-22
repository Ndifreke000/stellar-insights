export type EventHandler = (event: unknown) => void;

/**
 * Typed channel map for the realtime feed. Each channel name maps to the
 * payload shape emitted by the protocol schema. Extend this map as the
 * protocol schema grows; subscribers get typed payloads per channel.
 */
export interface ChannelMap {
  corridor: { corridor_id: string; seq: number; [key: string]: unknown };
  event: { event_id: string; seq: number; [key: string]: unknown };
  ledger: { ledger: number; seq: number; [key: string]: unknown };
}

export type ChannelName = keyof ChannelMap;

export type TypedHandler<C extends ChannelName> = (
  payload: ChannelMap[C],
) => void;

interface ConnectionKey {
  rpcUrl: string;
  network: string;
}

interface ManagedConnection {
  ws: WebSocket;
  refCount: number;
  handlers: Set<EventHandler>;
  lastSeq: number;
  reconnectAttempts: number;
  reconnectTimer: ReturnType<typeof setTimeout> | null;
  closed: boolean;
}

const connections = new Map<string, ManagedConnection>();

const BASE_BACKOFF_MS = 500;
const MAX_BACKOFF_MS = 30_000;

function connectionKey(key: ConnectionKey): string {
  return `${key.rpcUrl}|${key.network}`;
}

function backoffDelay(attempt: number): number {
  const exp = Math.min(MAX_BACKOFF_MS, BASE_BACKOFF_MS * 2 ** attempt);
  // Full jitter to avoid thundering herds on reconnect.
  return Math.floor(Math.random() * exp);
}

function buildUrl(key: ConnectionKey, sinceSeq: number): string {
  const wsUrl = key.rpcUrl.replace(/^http/, "ws");
  if (sinceSeq <= 0) return wsUrl;
  const sep = wsUrl.includes("?") ? "&" : "?";
  return `${wsUrl}${sep}since_seq=${sinceSeq}`;
}

function dispatch(conn: ManagedConnection, parsed: unknown): void {
  if (parsed && typeof parsed === "object" && "seq" in parsed) {
    const seq = (parsed as { seq: unknown }).seq;
    if (typeof seq === "number" && seq > conn.lastSeq) {
      conn.lastSeq = seq;
    }
  }
  for (const h of conn.handlers) {
    h(parsed);
  }
}

function connect(k: string, key: ConnectionKey, conn: ManagedConnection): void {
  const ws = new WebSocket(buildUrl(key, conn.lastSeq));
  conn.ws = ws;

  ws.onopen = () => {
    conn.reconnectAttempts = 0;
  };

  ws.onmessage = (event) => {
    let parsed: unknown;
    try {
      parsed = JSON.parse(String(event.data));
    } catch {
      return;
    }
    dispatch(conn, parsed);
  };

  ws.onclose = () => {
    if (conn.closed) {
      connections.delete(k);
      return;
    }
    scheduleReconnect(k, key, conn);
  };
}

function scheduleReconnect(
  k: string,
  key: ConnectionKey,
  conn: ManagedConnection,
): void {
  if (conn.reconnectTimer !== null) return;
  const delay = backoffDelay(conn.reconnectAttempts);
  conn.reconnectAttempts++;
  conn.reconnectTimer = setTimeout(() => {
    conn.reconnectTimer = null;
    if (conn.closed) return;
    connect(k, key, conn);
  }, delay);
}

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

  const conn: ManagedConnection = {
    ws: undefined as unknown as WebSocket,
    refCount: 1,
    handlers: new Set([handler]),
    lastSeq: 0,
    reconnectAttempts: 0,
    reconnectTimer: null,
    closed: false,
  };

  connections.set(k, conn);
  connect(k, key, conn);
}

/**
 * Subscribe to a typed channel. The handler receives payloads typed by the
 * channel map, and reconnection resumes from the last seen `seq`.
 */
export function subscribe<C extends ChannelName>(
  key: ConnectionKey,
  channel: C,
  handler: TypedHandler<C>,
): () => void {
  const wrapped: EventHandler = (event) => {
    if (!event || typeof event !== "object") return;
    const record = event as { channel?: unknown };
    if (record.channel !== undefined && record.channel !== channel) return;
    handler(event as ChannelMap[C]);
  };
  acquireConnection(key, wrapped);
  return () => releaseConnection(key, wrapped);
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
    conn.closed = true;
    if (conn.reconnectTimer !== null) {
      clearTimeout(conn.reconnectTimer);
      conn.reconnectTimer = null;
    }
    conn.ws.close();
    connections.delete(k);
  }
}

export function closeAllConnections(): void {
  for (const conn of connections.values()) {
    conn.closed = true;
    if (conn.reconnectTimer !== null) {
      clearTimeout(conn.reconnectTimer);
      conn.reconnectTimer = null;
    }
    conn.ws.close();
  }
  connections.clear();
}
