/**
 * WebSocket client for real-time updates from the Stellar Insights backend
 */

export type WsMessageType =
  | 'snapshot_update'
  | 'corridor_update'
  | 'anchor_update'
  | 'ping'
  | 'pong'
  | 'connected'
  | 'error';

export interface WsSnapshotUpdate {
  type: 'snapshot_update';
  snapshot_id: string;
  epoch: number;
  timestamp: string;
  hash: string;
}

export interface WsCorridorUpdate {
  type: 'corridor_update';
  corridor_id: string;
  corridor_key: string;
  success_rate: number;
  volume_usd: number;
  total_transactions: number;
}

export interface WsAnchorUpdate {
  type: 'anchor_update';
  anchor_id: string;
  name: string;
  reliability_score: number;
  status: string;
}

export interface WsPing {
  type: 'ping';
  timestamp: number;
}

export interface WsPong {
  type: 'pong';
  timestamp: number;
}

export interface WsConnected {
  type: 'connected';
  connection_id: string;
}

export interface WsError {
  type: 'error';
  message: string;
}

export type WsMessage =
  | WsSnapshotUpdate
  | WsCorridorUpdate
  | WsAnchorUpdate
  | WsPing
  | WsPong
  | WsConnected
  | WsError;

export type WsEventHandler = (message: WsMessage) => void;

export interface WebSocketConfig {
  url?: string;
  token?: string;
  autoReconnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
}

export class StellarInsightsWebSocket {
  private ws: WebSocket | null = null;
  private config: Required<WebSocketConfig>;
  private listeners: Map<WsMessageType, Set<WsEventHandler>> = new Map();
  private reconnectAttempts = 0;
  private reconnectTimeout: NodeJS.Timeout | null = null;
  private isManualClose = false;
  private connectionId: string | null = null;

  constructor(config: WebSocketConfig = {}) {
    this.config = {
      url: config.url || this.getDefaultWsUrl(),
      token: config.token || '',
      autoReconnect: config.autoReconnect ?? true,
      reconnectInterval: config.reconnectInterval || 3000,
      maxReconnectAttempts: config.maxReconnectAttempts || 5,
    };
  }

  /**
   * Get the default WebSocket URL based on the current environment
   */
  private getDefaultWsUrl(): string {
    if (typeof window === 'undefined') {
      return 'ws://localhost:8080/ws';
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.hostname;
    const port = process.env.NEXT_PUBLIC_WS_PORT || '8080';

    return `${protocol}//${host}:${port}/ws`;
  }

  /**
   * Connect to the WebSocket server
   */
  public connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      console.log('WebSocket already connected');
      return;
    }

    this.isManualClose = false;

    try {
      const url = new URL(this.config.url);
      if (this.config.token) {
        url.searchParams.set('token', this.config.token);
      }

      console.log('Connecting to WebSocket:', url.toString());
      this.ws = new WebSocket(url.toString());

      this.ws.onopen = this.handleOpen.bind(this);
      this.ws.onmessage = this.handleMessage.bind(this);
      this.ws.onerror = this.handleError.bind(this);
      this.ws.onclose = this.handleClose.bind(this);
    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
      this.scheduleReconnect();
    }
  }

  /**
   * Disconnect from the WebSocket server
   */
  public disconnect(): void {
    this.isManualClose = true;

    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.connectionId = null;
    console.log('WebSocket disconnected');
  }

  /**
   * Check if the WebSocket is connected
   */
  public isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  /**
   * Get the connection ID (if connected)
   */
  public getConnectionId(): string | null {
    return this.connectionId;
  }

  /**
   * Subscribe to a specific message type
   */
  public on(type: WsMessageType, handler: WsEventHandler): () => void {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set());
    }

    this.listeners.get(type)!.add(handler);

    // Return unsubscribe function
    return () => {
      this.listeners.get(type)?.delete(handler);
    };
  }

  /**
   * Subscribe to all message types
   */
  public onAny(handler: WsEventHandler): () => void {
    const unsubscribers: Array<() => void> = [];

    const types: WsMessageType[] = [
      'snapshot_update',
      'corridor_update',
      'anchor_update',
      'ping',
      'pong',
      'connected',
      'error',
    ];

    types.forEach((type) => {
      unsubscribers.push(this.on(type, handler));
    });

    // Return function to unsubscribe from all
    return () => {
      unsubscribers.forEach((unsub) => unsub());
    };
  }

  /**
   * Send a ping message
   */
  public ping(): void {
    if (!this.isConnected()) {
      console.warn('Cannot send ping: WebSocket not connected');
      return;
    }

    const message: WsPing = {
      type: 'ping',
      timestamp: Date.now(),
    };

    this.send(message);
  }

  /**
   * Send a message to the server
   */
  private send(message: WsPing | WsPong): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('Cannot send message: WebSocket not connected');
      return;
    }

    try {
      this.ws.send(JSON.stringify(message));
    } catch (error) {
      console.error('Failed to send message:', error);
    }
  }

  /**
   * Handle WebSocket open event
   */
  private handleOpen(): void {
    console.log('WebSocket connected');
    this.reconnectAttempts = 0;

    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
  }

  /**
   * Handle incoming WebSocket messages
   */
  private handleMessage(event: MessageEvent): void {
    try {
      const message = JSON.parse(event.data) as WsMessage;

      // Store connection ID when connected
      if (message.type === 'connected') {
        this.connectionId = message.connection_id;
      }

      // Automatically respond to pings with pongs
      if (message.type === 'ping') {
        const pong: WsPong = {
          type: 'pong',
          timestamp: message.timestamp,
        };
        this.send(pong);
      }

      // Notify all registered listeners
      const handlers = this.listeners.get(message.type);
      if (handlers) {
        handlers.forEach((handler) => {
          try {
            handler(message);
          } catch (error) {
            console.error('Error in message handler:', error);
          }
        });
      }
    } catch (error) {
      console.error('Failed to parse WebSocket message:', error);
    }
  }

  /**
   * Handle WebSocket error event
   */
  private handleError(error: Event): void {
    console.error('WebSocket error:', error);

    const errorMessage: WsError = {
      type: 'error',
      message: 'WebSocket connection error',
    };

    const handlers = this.listeners.get('error');
    if (handlers) {
      handlers.forEach((handler) => handler(errorMessage));
    }
  }

  /**
   * Handle WebSocket close event
   */
  private handleClose(event: CloseEvent): void {
    console.log(
      `WebSocket closed: code=${event.code}, reason=${event.reason || 'none'}`
    );

    this.connectionId = null;

    if (!this.isManualClose && this.config.autoReconnect) {
      this.scheduleReconnect();
    }
  }

  /**
   * Schedule a reconnection attempt
   */
  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.config.maxReconnectAttempts) {
      console.error(
        `Max reconnection attempts (${this.config.maxReconnectAttempts}) reached`
      );
      return;
    }

    this.reconnectAttempts++;

    const delay = this.config.reconnectInterval * this.reconnectAttempts;
    console.log(
      `Scheduling reconnection attempt ${this.reconnectAttempts} in ${delay}ms`
    );

    this.reconnectTimeout = setTimeout(() => {
      this.connect();
    }, delay);
  }
}

// Singleton instance for easy access
let wsInstance: StellarInsightsWebSocket | null = null;

/**
 * Get the singleton WebSocket instance
 */
export function getWebSocketInstance(
  config?: WebSocketConfig
): StellarInsightsWebSocket {
  if (!wsInstance) {
    wsInstance = new StellarInsightsWebSocket(config);
  }
  return wsInstance;
}

/**
 * Reset the WebSocket singleton (useful for testing)
 */
export function resetWebSocketInstance(): void {
  if (wsInstance) {
    wsInstance.disconnect();
    wsInstance = null;
  }
}
