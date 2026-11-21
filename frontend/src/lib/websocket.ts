/**
 * Single shared realtime WebSocket client for the entire browser tab (Issue #2834).
 *
 * Replaces duplicate connection management across `lib/websocket.ts`,
 * `hooks/useWebSocket.ts`, and `hooks/useRealtimeCollaboration.ts` with one
 * connection manager singleton featuring:
 * - Reference-counted consumers and channel subscriptions
 * - Jittered exponential backoff reconnection
 * - Page Visibility (`visibilitychange`) and network (`online`/`offline`) awareness
 * - Per-channel sequence tracking (`last_seq`) for lossless server replay
 * - External-store snapshot API compatible with React's `useSyncExternalStore`
 */
import { logger } from "@/lib/logger";

export enum ConnectionState {
  DISCONNECTED = "DISCONNECTED",
  CONNECTING = "CONNECTING",
  CONNECTED = "CONNECTED",
  RECONNECTING = "RECONNECTING",
}

export type WsMessageType =
  | "snapshot_update"
  | "corridor_update"
  | "anchor_update"
  | "new_payment"
  | "transaction_update"
  | "health_alert"
  | "subscription_confirm"
  | "collaboration_message"
  | "collaboration_presence"
  | "ping"
  | "pong"
  | "connected"
  | "error"
  | string;

export interface WsSnapshotUpdate {
  type: "snapshot_update";
  snapshot_id: string;
  epoch: number;
  timestamp: string;
  hash: string;
}

export interface WsCorridorUpdate {
  type: "corridor_update";
  corridor_id?: string;
  corridor_key: string;
  asset_a_code?: string;
  asset_a_issuer?: string;
  asset_b_code?: string;
  asset_b_issuer?: string;
  success_rate?: number;
  health_score?: number;
  volume_usd?: number;
  liquidity_depth_usd?: number;
  liquidity_volume_24h_usd?: number;
  average_latency_ms?: number;
  total_transactions?: number;
  last_updated?: string;
}

export interface WsAnchorUpdate {
  type: "anchor_update";
  anchor_id: string;
  name: string;
  reliability_score: number;
  status: string;
}

export interface WsPing {
  type: "ping";
  timestamp?: number;
}

export interface WsPong {
  type: "pong";
  timestamp?: number;
}

export interface WsConnected {
  type: "connected";
  connection_id: string;
}

export interface WsError {
  type: "error";
  message: string;
}

export interface WsGenericMessage {
  type: string;
  channel?: string;
  seq?: number;
  [key: string]: unknown;
}

export type WsMessage = WsGenericMessage;

export type WsEventHandler = (message: WsMessage) => void;
export type WsLifecycleHandler = () => void;
export type WsErrorHandler = (error: Event) => void;
export type StateChangeListener = () => void;

export interface WebSocketConfig {
  url?: string;
  token?: string;
  autoReconnect?: boolean;
  /** Base reconnect interval in ms (default: 1000). */
  reconnectInterval?: number;
  /** Maximum backoff delay cap in ms (default: 30000). */
  maxReconnectDelay?: number;
  /** Exponential growth multiplier (default: 1.5). */
  backoffFactor?: number;
  /** Maximum consecutive reconnect attempts before pausing (default: 10). */
  maxReconnectAttempts?: number;
  /** Optional RNG override for deterministic jitter tests. */
  random?: () => number;
}

export interface RealtimeConnectionSnapshot {
  state: ConnectionState;
  isConnected: boolean;
  isConnecting: boolean;
  connectionAttempts: number;
  connectionId: string | null;
  activeChannels: string[];
  lastMessage: WsMessage | null;
}

/**
 * Compute an equal-jittered exponential backoff delay in milliseconds.
 *
 * Equal jitter keeps delays within `[base * 0.5, base]` so reconnecting tabs
 * spread out after a server restart while still respecting a lower bound.
 */
export function computeJitteredBackoffDelay(
  attempt: number,
  baseIntervalMs: number,
  maxDelayMs = 30_000,
  backoffFactor = 1.5,
  randomFn: () => number = Math.random,
): number {
  const safeAttempt = Math.max(1, attempt);
  const exponential = Math.min(
    maxDelayMs,
    baseIntervalMs * Math.pow(backoffFactor, safeAttempt - 1),
  );
  const clampedRandom = Math.min(1, Math.max(0, randomFn()));
  return Math.max(1, Math.round(exponential * (0.5 + clampedRandom * 0.5)));
}

/**
 * Resolve the canonical default WebSocket URL for the current environment.
 */
export function getDefaultWsUrl(): string {
  const envUrl = process.env.NEXT_PUBLIC_WS_URL?.trim();
  if (envUrl) {
    return envUrl;
  }
  if (typeof window === "undefined") {
    return "ws://localhost:8080/ws";
  }
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const host = window.location.hostname || "localhost";
  const port = process.env.NEXT_PUBLIC_WS_PORT || "8080";
  return `${protocol}//${host}:${port}/ws`;
}

/**
 * Normalize a WebSocket URL so equivalent callers (e.g. `""`, `undefined`,
 * `"ws://localhost:8080/ws"`, or `NEXT_PUBLIC_WS_URL`) share one singleton key.
 */
export function normalizeWsUrl(url?: string): string {
  const trimmed = url?.trim();
  if (!trimmed) {
    return getDefaultWsUrl();
  }
  return trimmed.replace(/\/+$/, "");
}

interface InternalConfig {
  url: string;
  token: string;
  autoReconnect: boolean;
  reconnectInterval: number;
  maxReconnectDelay: number;
  backoffFactor: number;
  maxReconnectAttempts: number;
  random: () => number;
}

export class RealtimeClient {
  private ws: WebSocket | null = null;
  private config: InternalConfig;

  private typeListeners: Map<string, Set<WsEventHandler>> = new Map();
  private anyListeners: Set<WsEventHandler> = new Set();
  private openListeners: Set<WsLifecycleHandler> = new Set();
  private closeListeners: Set<WsLifecycleHandler> = new Set();
  private errorListeners: Set<WsErrorHandler> = new Set();
  private stateListeners: Set<StateChangeListener> = new Set();

  /** Reference counts per subscribed channel name. */
  private channelRefCounts: Map<string, number> = new Map();
  /** Highest sequence number observed per channel for lossless server replay. */
  private channelLastSeq: Map<string, number> = new Map();

  private consumerCount = 0;
  private reconnectAttempts = 0;
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  private isManualClose = false;
  private isHidden = false;
  private isOffline = false;
  private connectionId: string | null = null;
  private state: ConnectionState = ConnectionState.DISCONNECTED;
  private lastMessage: WsMessage | null = null;
  private snapshot: RealtimeConnectionSnapshot;

  private visibilityHandler: (() => void) | null = null;
  private onlineHandler: (() => void) | null = null;
  private offlineHandler: (() => void) | null = null;

  constructor(config: WebSocketConfig = {}) {
    this.config = {
      url: normalizeWsUrl(config.url),
      token: config.token ?? "",
      autoReconnect: config.autoReconnect ?? true,
      reconnectInterval: config.reconnectInterval ?? 1000,
      maxReconnectDelay: config.maxReconnectDelay ?? 30_000,
      backoffFactor: config.backoffFactor ?? 1.5,
      maxReconnectAttempts: config.maxReconnectAttempts ?? 10,
      random: config.random ?? Math.random,
    };

    this.snapshot = this.buildSnapshot();
    this.bindEnvironmentListeners();
  }

  /** Allow subsequent hooks with custom retry settings to refine timing (e.g. fast test intervals). */
  public configure(overrides: Partial<WebSocketConfig>): void {
    if (overrides.reconnectInterval !== undefined) {
      this.config.reconnectInterval = overrides.reconnectInterval;
    }
    if (overrides.maxReconnectDelay !== undefined) {
      this.config.maxReconnectDelay = overrides.maxReconnectDelay;
    }
    if (overrides.backoffFactor !== undefined) {
      this.config.backoffFactor = overrides.backoffFactor;
    }
    if (overrides.maxReconnectAttempts !== undefined) {
      this.config.maxReconnectAttempts = overrides.maxReconnectAttempts;
    }
    if (overrides.autoReconnect !== undefined) {
      this.config.autoReconnect = overrides.autoReconnect;
    }
    if (overrides.random !== undefined) {
      this.config.random = overrides.random;
    }
  }

  public getUrl(): string {
    return this.config.url;
  }

  public getConsumerCount(): number {
    return this.consumerCount;
  }

  public getChannelRefCount(channel: string): number {
    return this.channelRefCounts.get(channel) ?? 0;
  }

  public getActiveChannels(): string[] {
    return Array.from(this.channelRefCounts.keys());
  }

  public isConnected(): boolean {
    return this.state === ConnectionState.CONNECTED;
  }

  public getConnectionId(): string | null {
    return this.connectionId;
  }

  public getSnapshot = (): RealtimeConnectionSnapshot => {
    return this.snapshot;
  };

  public subscribeState = (listener: StateChangeListener): (() => void) => {
    this.stateListeners.add(listener);
    return () => {
      this.stateListeners.delete(listener);
    };
  };

  /**
   * Register a React component/hook consumer. Opens the shared socket when the
   * first consumer attaches and closes it cleanly when the last consumer detaches.
   */
  public attach(): () => void {
    this.consumerCount += 1;
    if (this.consumerCount === 1) {
      this.connect();
    } else if (
      this.state === ConnectionState.DISCONNECTED &&
      !this.isHidden &&
      !this.isOffline
    ) {
      this.connect();
    }

    let detached = false;
    return () => {
      if (detached) return;
      detached = true;
      this.consumerCount = Math.max(0, this.consumerCount - 1);
      if (this.consumerCount === 0) {
        this.disconnect();
      }
    };
  }

  /**
   * Increment reference counts for `channels`. Sends a wire `subscribe` frame
   * ONLY for channels whose ref count transitioned from `0` to `1`.
   */
  public acquireChannels(channels: string[]): string[] {
    const unique = Array.from(
      new Set(channels.map((c) => c.trim()).filter(Boolean)),
    );
    if (unique.length === 0) return [];

    const newlyAdded: string[] = [];
    for (const channel of unique) {
      const prev = this.channelRefCounts.get(channel) ?? 0;
      this.channelRefCounts.set(channel, prev + 1);
      if (prev === 0) {
        newlyAdded.push(channel);
      }
    }

    if (newlyAdded.length > 0) {
      this.emitSnapshot();
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.sendSubscribeFrame(newlyAdded);
      }
    }
    return newlyAdded;
  }

  /**
   * Decrement reference counts for `channels`. Sends a wire `unsubscribe` frame
   * ONLY for channels whose ref count transitioned from `1` to `0`.
   */
  public releaseChannels(channels: string[]): string[] {
    const unique = Array.from(
      new Set(channels.map((c) => c.trim()).filter(Boolean)),
    );
    if (unique.length === 0) return [];

    const removed: string[] = [];
    for (const channel of unique) {
      const prev = this.channelRefCounts.get(channel) ?? 0;
      if (prev <= 1) {
        if (prev === 1) {
          this.channelRefCounts.delete(channel);
          this.channelLastSeq.delete(channel);
          removed.push(channel);
        }
      } else {
        this.channelRefCounts.set(channel, prev - 1);
      }
    }

    if (removed.length > 0) {
      this.emitSnapshot();
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.send({ type: "unsubscribe", channels: removed });
      }
    }
    return removed;
  }

  /**
   * Open the underlying WebSocket if not already open or connecting.
   */
  public connect(): void {
    if (typeof WebSocket === "undefined") {
      return;
    }

    if (
      this.ws &&
      (this.ws.readyState === WebSocket.OPEN ||
        this.ws.readyState === WebSocket.CONNECTING)
    ) {
      return;
    }

    this.clearReconnectTimer();
    this.isManualClose = false;
    this.setState(ConnectionState.CONNECTING);

    try {
      let targetUrl = this.config.url;
      if (this.config.token) {
        const parsed = new URL(targetUrl);
        parsed.searchParams.set("token", this.config.token);
        targetUrl = parsed.toString();
      }

      logger.debug("RealtimeClient connecting", { url: targetUrl });
      const socket = new WebSocket(targetUrl);
      this.ws = socket;

      socket.onopen = () => {
        if (this.ws !== socket) return;
        this.handleOpen(socket);
      };
      socket.onmessage = (event) => {
        if (this.ws !== socket) return;
        this.handleMessage(event);
      };
      socket.onerror = (event) => {
        if (this.ws !== socket) return;
        this.handleError(event);
      };
      socket.onclose = (event) => {
        if (this.ws !== socket) return;
        this.handleClose(event);
      };
    } catch (error) {
      logger.error("RealtimeClient failed to create WebSocket:", error);
      this.setState(ConnectionState.DISCONNECTED);
      if (this.config.autoReconnect && !this.isManualClose) {
        this.scheduleReconnect();
      }
    }
  }

  /**
   * Close the underlying socket and cancel any pending reconnect timer.
   */
  public disconnect(): void {
    this.isManualClose = true;
    this.clearReconnectTimer();

    if (this.ws) {
      const socket = this.ws;
      this.ws = null;
      socket.onopen = null;
      socket.onclose = null;
      socket.onerror = null;
      socket.onmessage = null;
      try {
        socket.close();
      } catch {
        // Ignore close errors on already-closed sockets
      }
    }

    this.connectionId = null;
    this.setState(ConnectionState.DISCONNECTED);
  }

  /**
   * Force a single reconnect cycle, resetting backoff attempts.
   */
  public reconnect(): void {
    this.disconnect();
    this.isManualClose = false;
    this.reconnectAttempts = 0;
    this.emitSnapshot();
    this.connect();
  }

  /**
   * Send a JSON payload over the shared socket if connected.
   */
  public send(message: WsMessage): boolean {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      logger.warn("RealtimeClient cannot send while disconnected", {
        type: message.type,
      });
      return false;
    }

    try {
      this.ws.send(JSON.stringify(message));
      return true;
    } catch (error) {
      logger.error("RealtimeClient failed to send message:", error);
      return false;
    }
  }

  public ping(): void {
    this.send({ type: "ping", timestamp: Date.now() });
  }

  /**
   * Subscribe to messages of a specific `type`.
   */
  public on(type: WsMessageType, handler: WsEventHandler): () => void {
    let bucket = this.typeListeners.get(type);
    if (!bucket) {
      bucket = new Set();
      this.typeListeners.set(type, bucket);
    }
    bucket.add(handler);
    return () => {
      bucket?.delete(handler);
      if (bucket && bucket.size === 0) {
        this.typeListeners.delete(type);
      }
    };
  }

  /**
   * Subscribe to every incoming WebSocket message.
   */
  public onAny(handler: WsEventHandler): () => void {
    this.anyListeners.add(handler);
    return () => {
      this.anyListeners.delete(handler);
    };
  }

  public onOpen(handler: WsLifecycleHandler): () => void {
    this.openListeners.add(handler);
    return () => {
      this.openListeners.delete(handler);
    };
  }

  public onClose(handler: WsLifecycleHandler): () => void {
    this.closeListeners.add(handler);
    return () => {
      this.closeListeners.delete(handler);
    };
  }

  public onError(handler: WsErrorHandler): () => void {
    this.errorListeners.add(handler);
    return () => {
      this.errorListeners.delete(handler);
    };
  }

  /**
   * Tear down DOM event listeners and close the socket (used in tests / reset).
   */
  public destroy(): void {
    this.disconnect();
    this.unbindEnvironmentListeners();
    this.typeListeners.clear();
    this.anyListeners.clear();
    this.openListeners.clear();
    this.closeListeners.clear();
    this.errorListeners.clear();
    this.stateListeners.clear();
    this.channelRefCounts.clear();
    this.channelLastSeq.clear();
    this.consumerCount = 0;
  }

  // ── Internal handlers ───────────────────────────────────────────────────────

  private handleOpen(socket: WebSocket): void {
    logger.debug("RealtimeClient connected");
    this.clearReconnectTimer();
    this.reconnectAttempts = 0;
    this.setState(ConnectionState.CONNECTED);

    // Restore all active ref-counted channels in one batched subscribe frame.
    const activeChannels = this.getActiveChannels();
    if (activeChannels.length > 0 && socket.readyState === WebSocket.OPEN) {
      this.sendSubscribeFrame(activeChannels);
    }

    this.openListeners.forEach((handler) => {
      try {
        handler();
      } catch (error) {
        logger.error("Error in WebSocket onOpen handler:", error);
      }
    });
  }

  private handleMessage(event: MessageEvent): void {
    try {
      const raw = JSON.parse(event.data as string) as WsMessage;

      if (typeof raw.channel === "string" && typeof raw.seq === "number") {
        const prev = this.channelLastSeq.get(raw.channel) ?? 0;
        if (raw.seq > prev) {
          this.channelLastSeq.set(raw.channel, raw.seq);
        }
      }

      // Unwrap Sequenced / Replayed envelopes from the backend while retaining
      // channel and seq metadata so downstream typed listeners receive the inner event.
      const message: WsMessage =
        (raw.type === "sequenced" || raw.type === "replayed") &&
        raw.inner &&
        typeof raw.inner === "object" &&
        typeof (raw.inner as WsMessage).type === "string"
          ? {
              ...(raw.inner as WsMessage),
              channel: raw.channel,
              seq: raw.seq,
            }
          : raw;

      if (message.type === "connected" && typeof message.connection_id === "string") {
        this.connectionId = message.connection_id;
      }

      if (message.type === "ping") {
        this.send({
          type: "pong",
          timestamp: typeof message.timestamp === "number" ? message.timestamp : Date.now(),
        });
      }

      this.lastMessage = message;
      this.emitSnapshot();

      const typed = this.typeListeners.get(message.type);
      if (typed) {
        typed.forEach((handler) => {
          try {
            handler(message);
          } catch (error) {
            logger.error("Error in typed WebSocket message handler:", error);
          }
        });
      }

      this.anyListeners.forEach((handler) => {
        try {
          handler(message);
        } catch (error) {
          logger.error("Error in WebSocket message handler:", error);
        }
      });
    } catch (error) {
      logger.error("Failed to parse WebSocket message:", error);
    }
  }

  private handleError(event: Event): void {
    logger.error("RealtimeClient WebSocket error:", event);

    this.errorListeners.forEach((handler) => {
      try {
        handler(event);
      } catch (err) {
        logger.error("Error in WebSocket onError handler:", err);
      }
    });

    const errHandlers = this.typeListeners.get("error");
    if (errHandlers) {
      const errorMsg: WsError = {
        type: "error",
        message: "WebSocket connection error",
      };
      errHandlers.forEach((handler) => {
        try {
          handler(errorMsg as unknown as WsMessage);
        } catch (err) {
          logger.error("Error in WebSocket error listener:", err);
        }
      });
    }
  }

  private handleClose(event?: CloseEvent): void {
    logger.debug("RealtimeClient closed", {
      code: event?.code,
      reason: event?.reason || "none",
    });

    this.ws = null;
    this.connectionId = null;

    this.closeListeners.forEach((handler) => {
      try {
        handler();
      } catch (err) {
        logger.error("Error in WebSocket onClose handler:", err);
      }
    });

    const shouldReconnect =
      !this.isManualClose &&
      this.config.autoReconnect &&
      this.reconnectAttempts < this.config.maxReconnectAttempts;

    if (shouldReconnect && !this.isHidden && !this.isOffline) {
      this.scheduleReconnect();
    } else {
      this.setState(ConnectionState.DISCONNECTED);
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimeout !== null) {
      // A single reconnect is already scheduled — never duplicate timers.
      return;
    }
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      logger.warn(
        `RealtimeClient max reconnect attempts (${this.config.maxReconnectAttempts}) reached`,
      );
      this.setState(ConnectionState.DISCONNECTED);
      return;
    }

    this.reconnectAttempts += 1;
    const delay = computeJitteredBackoffDelay(
      this.reconnectAttempts,
      this.config.reconnectInterval,
      this.config.maxReconnectDelay,
      this.config.backoffFactor,
      this.config.random,
    );

    this.setState(ConnectionState.RECONNECTING);
    logger.debug("RealtimeClient scheduling reconnect", {
      attempt: this.reconnectAttempts,
      delayMs: delay,
    });

    this.reconnectTimeout = setTimeout(() => {
      this.reconnectTimeout = null;
      if (!this.isManualClose && !this.isHidden && !this.isOffline) {
        this.connect();
      }
    }, delay);
  }

  private sendSubscribeFrame(channels: string[]): void {
    const lastSeq: Record<string, number> = {};
    let hasSeq = false;
    for (const ch of channels) {
      const seq = this.channelLastSeq.get(ch);
      if (seq !== undefined && seq > 0) {
        lastSeq[ch] = seq;
        hasSeq = true;
      }
    }

    const payload: WsMessage = hasSeq
      ? { type: "subscribe", channels, last_seq: lastSeq }
      : { type: "subscribe", channels };

    this.send(payload);
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimeout !== null) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
  }

  private setState(next: ConnectionState): void {
    this.state = next;
    this.emitSnapshot();
  }

  private buildSnapshot(): RealtimeConnectionSnapshot {
    return {
      state: this.state,
      isConnected: this.state === ConnectionState.CONNECTED,
      isConnecting:
        this.state === ConnectionState.CONNECTING ||
        this.state === ConnectionState.RECONNECTING,
      connectionAttempts: this.reconnectAttempts,
      connectionId: this.connectionId,
      activeChannels: this.getActiveChannels(),
      lastMessage: this.lastMessage,
    };
  }

  private emitSnapshot(): void {
    this.snapshot = this.buildSnapshot();
    this.stateListeners.forEach((listener) => {
      try {
        listener();
      } catch (error) {
        logger.error("Error in WebSocket state listener:", error);
      }
    });
  }

  private bindEnvironmentListeners(): void {
    if (typeof document !== "undefined") {
      this.isHidden = document.visibilityState === "hidden";
      this.visibilityHandler = () => {
        const nextHidden = document.visibilityState === "hidden";
        if (nextHidden === this.isHidden) return;
        this.isHidden = nextHidden;

        if (nextHidden) {
          logger.debug("Tab hidden: pausing WebSocket reconnect timer");
          this.clearReconnectTimer();
          if (this.state === ConnectionState.RECONNECTING) {
            this.setState(ConnectionState.DISCONNECTED);
          }
        } else {
          logger.debug("Tab visible: resuming WebSocket connection");
          if (
            !this.isManualClose &&
            !this.isOffline &&
            this.state === ConnectionState.DISCONNECTED &&
            (this.consumerCount > 0 || this.channelRefCounts.size > 0)
          ) {
            this.scheduleReconnect();
          }
        }
      };
      document.addEventListener("visibilitychange", this.visibilityHandler);
    }

    if (typeof window !== "undefined") {
      this.isOffline = typeof navigator !== "undefined" && navigator.onLine === false;
      this.offlineHandler = () => {
        this.isOffline = true;
        this.clearReconnectTimer();
      };
      this.onlineHandler = () => {
        this.isOffline = false;
        if (
          !this.isManualClose &&
          !this.isHidden &&
          this.state === ConnectionState.DISCONNECTED &&
          (this.consumerCount > 0 || this.channelRefCounts.size > 0)
        ) {
          this.reconnectAttempts = 0;
          this.scheduleReconnect();
        }
      };
      window.addEventListener("offline", this.offlineHandler);
      window.addEventListener("online", this.onlineHandler);
    }
  }

  private unbindEnvironmentListeners(): void {
    if (typeof document !== "undefined" && this.visibilityHandler) {
      document.removeEventListener("visibilitychange", this.visibilityHandler);
      this.visibilityHandler = null;
    }
    if (typeof window !== "undefined") {
      if (this.offlineHandler) {
        window.removeEventListener("offline", this.offlineHandler);
        this.offlineHandler = null;
      }
      if (this.onlineHandler) {
        window.removeEventListener("online", this.onlineHandler);
        this.onlineHandler = null;
      }
    }
  }
}

// Backward-compatible class export name.
export { RealtimeClient as PayRaiderWebSocket };

// ── Singleton registry ────────────────────────────────────────────────────────

const clientRegistry = new Map<string, RealtimeClient>();

/**
 * Return the singleton [`RealtimeClient`] for `config.url` (or the default tab
 * WebSocket URL when omitted). All hooks sharing the same normalized URL reuse
 * exactly one WebSocket connection.
 */
export function getRealtimeClient(config?: WebSocketConfig): RealtimeClient {
  const key = normalizeWsUrl(config?.url);
  let instance = clientRegistry.get(key);
  if (!instance) {
    instance = new RealtimeClient({ ...config, url: key });
    clientRegistry.set(key, instance);
  } else if (config) {
    instance.configure(config);
  }
  return instance;
}

/**
 * Backward-compatible alias for [`getRealtimeClient`].
 */
export function getWebSocketInstance(config?: WebSocketConfig): RealtimeClient {
  return getRealtimeClient(config);
}

/**
 * Destroy all cached realtime client singletons (intended for test cleanup).
 */
export function resetRealtimeClient(): void {
  clientRegistry.forEach((client) => client.destroy());
  clientRegistry.clear();
}

/**
 * Backward-compatible alias for [`resetRealtimeClient`].
 */
export function resetWebSocketInstance(): void {
  resetRealtimeClient();
}
