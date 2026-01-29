'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import {
  getWebSocketInstance,
  type WsMessage,
  type WsMessageType,
  type WebSocketConfig,
} from '@/lib/websocket';

export interface UseWebSocketOptions extends WebSocketConfig {
  /**
   * Automatically connect on mount
   * @default true
   */
  autoConnect?: boolean;

  /**
   * Specific message types to listen for (if not provided, listens to all)
   */
  messageTypes?: WsMessageType[];

  /**
   * Callback for when messages are received
   */
  onMessage?: (message: WsMessage) => void;

  /**
   * Callback for when connection is established
   */
  onConnect?: () => void;

  /**
   * Callback for when connection is closed
   */
  onDisconnect?: () => void;

  /**
   * Callback for when an error occurs
   */
  onError?: (error: string) => void;
}

export interface UseWebSocketReturn {
  /**
   * Whether the WebSocket is currently connected
   */
  isConnected: boolean;

  /**
   * The connection ID (if connected)
   */
  connectionId: string | null;

  /**
   * Most recent message received
   */
  lastMessage: WsMessage | null;

  /**
   * Manually connect to the WebSocket
   */
  connect: () => void;

  /**
   * Manually disconnect from the WebSocket
   */
  disconnect: () => void;

  /**
   * Send a ping to the server
   */
  ping: () => void;
}

/**
 * React hook for using the Stellar Insights WebSocket connection
 */
export function useWebSocket(
  options: UseWebSocketOptions = {}
): UseWebSocketReturn {
  const {
    autoConnect = true,
    messageTypes,
    onMessage,
    onConnect,
    onDisconnect,
    onError,
    ...wsConfig
  } = options;

  const [isConnected, setIsConnected] = useState(false);
  const [connectionId, setConnectionId] = useState<string | null>(null);
  const [lastMessage, setLastMessage] = useState<WsMessage | null>(null);

  const wsRef = useRef(getWebSocketInstance(wsConfig));
  const unsubscribersRef = useRef<Array<() => void>>([]);

  // Handle message callback
  const handleMessage = useCallback(
    (message: WsMessage) => {
      setLastMessage(message);

      // Update connection state
      if (message.type === 'connected') {
        setIsConnected(true);
        setConnectionId(message.connection_id);
        onConnect?.();
      }

      // Handle errors
      if (message.type === 'error') {
        onError?.(message.message);
      }

      // Call user-provided message handler
      onMessage?.(message);
    },
    [onMessage, onConnect, onError]
  );

  // Connect to WebSocket
  const connect = useCallback(() => {
    const ws = wsRef.current;

    // Clear any existing subscriptions
    unsubscribersRef.current.forEach((unsub) => unsub());
    unsubscribersRef.current = [];

    // Subscribe to messages
    if (messageTypes && messageTypes.length > 0) {
      messageTypes.forEach((type) => {
        const unsub = ws.on(type, handleMessage);
        unsubscribersRef.current.push(unsub);
      });
    } else {
      // Subscribe to all message types
      const unsub = ws.onAny(handleMessage);
      unsubscribersRef.current.push(unsub);
    }

    // Connect
    ws.connect();

    // Check connection status periodically
    const checkInterval = setInterval(() => {
      const connected = ws.isConnected();
      setIsConnected(connected);

      if (connected) {
        setConnectionId(ws.getConnectionId());
      } else {
        setConnectionId(null);
      }
    }, 1000);

    // Store interval for cleanup
    unsubscribersRef.current.push(() => {
      clearInterval(checkInterval);
    });
  }, [messageTypes, handleMessage]);

  // Disconnect from WebSocket
  const disconnect = useCallback(() => {
    const ws = wsRef.current;

    // Unsubscribe from all events
    unsubscribersRef.current.forEach((unsub) => unsub());
    unsubscribersRef.current = [];

    // Disconnect
    ws.disconnect();

    setIsConnected(false);
    setConnectionId(null);

    onDisconnect?.();
  }, [onDisconnect]);

  // Send ping
  const ping = useCallback(() => {
    wsRef.current.ping();
  }, []);

  // Auto-connect on mount
  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    // Cleanup on unmount
    return () => {
      disconnect();
    };
  }, [autoConnect, connect, disconnect]);

  return {
    isConnected,
    connectionId,
    lastMessage,
    connect,
    disconnect,
    ping,
  };
}

/**
 * Hook for listening to snapshot updates
 */
export function useSnapshotUpdates(
  onUpdate: (update: Extract<WsMessage, { type: 'snapshot_update' }>) => void
) {
  return useWebSocket({
    messageTypes: ['snapshot_update'],
    onMessage: (message) => {
      if (message.type === 'snapshot_update') {
        onUpdate(message);
      }
    },
  });
}

/**
 * Hook for listening to corridor updates
 */
export function useCorridorUpdates(
  onUpdate: (update: Extract<WsMessage, { type: 'corridor_update' }>) => void
) {
  return useWebSocket({
    messageTypes: ['corridor_update'],
    onMessage: (message) => {
      if (message.type === 'corridor_update') {
        onUpdate(message);
      }
    },
  });
}

/**
 * Hook for listening to anchor updates
 */
export function useAnchorUpdates(
  onUpdate: (update: Extract<WsMessage, { type: 'anchor_update' }>) => void
) {
  return useWebSocket({
    messageTypes: ['anchor_update'],
    onMessage: (message) => {
      if (message.type === 'anchor_update') {
        onUpdate(message);
      }
    },
  });
}
