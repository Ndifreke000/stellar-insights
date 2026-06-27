import { useEffect, useRef, useState, useCallback } from "react";
import { logger } from "@/lib/logger";

export enum ConnectionState {
  DISCONNECTED = "DISCONNECTED",
  CONNECTING = "CONNECTING",
  CONNECTED = "CONNECTED",
  RECONNECTING = "RECONNECTING",
}

export interface WsMessage {
  type: string;
  [key: string]: string | number | boolean | null | undefined | string[] | Record<string, unknown>;
}

export interface UseWebSocketOptions {
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (error: Event) => void;
  onMessage?: (message: WsMessage) => void;
}

export interface UseWebSocketReturn {
  isConnected: boolean;
  isConnecting: boolean;
  lastMessage: WsMessage | null;
  connectionAttempts: number;
  send: (message: WsMessage) => void;
  subscribe: (channels: string[]) => void;
  unsubscribe: (channels: string[]) => void;
  reconnect: () => void;
}

export function useWebSocket(
  url: string,
  options: UseWebSocketOptions = {},
): UseWebSocketReturn {
  const {
    reconnectInterval = 3000,
    maxReconnectAttempts = 5,
    onOpen,
    onClose,
    onError,
    onMessage,
  } = options;

  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [lastMessage, setLastMessage] = useState<WsMessage | null>(null);
  const [connectionAttempts, setConnectionAttempts] = useState(0);
  const [connectionState, setConnectionState] = useState<ConnectionState>(
    ConnectionState.DISCONNECTED
  );

  const [appState, setAppState] = useState<"active" | "background">("active");
  const appStateRef = useRef<"active" | "background">("active");

  useEffect(() => {
    appStateRef.current = appState;
  }, [appState]);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const shouldReconnectRef = useRef(true);
  const isConnectingRef = useRef(false);
  const connectionAttemptsRef = useRef(0);
  const optionsRef = useRef({ onOpen, onClose, onError, onMessage });
  optionsRef.current = { onOpen, onClose, onError, onMessage };

  const connect = useCallback(() => {
    if (isConnectingRef.current) {
      return;
    }

    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    // Close any lingering socket before creating a new one
    if (wsRef.current && wsRef.current.readyState !== WebSocket.CLOSED) {
      wsRef.current.onclose = null;
      wsRef.current.onerror = null;
      wsRef.current.onmessage = null;
      wsRef.current.close();
      wsRef.current = null;
    }

    isConnectingRef.current = true;
    setIsConnecting(true);

    try {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        logger.debug("WebSocket connected");
        setIsConnected(true);
        setIsConnecting(false);
        setConnectionState(ConnectionState.CONNECTED);
        connectionAttemptsRef.current = 0;
        setConnectionAttempts(0);
        isConnectingRef.current = false;
        optionsRef.current.onOpen?.();
      };

      ws.onclose = () => {
        logger.debug("WebSocket disconnected");
        setIsConnected(false);
        setIsConnecting(false);
        isConnectingRef.current = false;
        optionsRef.current.onClose?.();

        if (
          shouldReconnectRef.current &&
          connectionAttemptsRef.current < maxReconnectAttempts &&
          appStateRef.current !== "background"
        ) {
          connectionAttemptsRef.current += 1;
          setConnectionAttempts(connectionAttemptsRef.current);
          setConnectionState(ConnectionState.RECONNECTING);
          reconnectTimeoutRef.current = setTimeout(
            () => {
              connect();
            },
            reconnectInterval * Math.pow(1.5, connectionAttemptsRef.current),
          );
        }
      };

      ws.onerror = (error) => {
        logger.error("WebSocket error:", error);
        setIsConnecting(false);
        isConnectingRef.current = false;
        setConnectionState(ConnectionState.DISCONNECTED);
        optionsRef.current.onError?.(error);
      };

      ws.onmessage = (event) => {
        try {
          const message: WsMessage = JSON.parse(event.data);
          setLastMessage(message);
          optionsRef.current.onMessage?.(message);
        } catch (error) {
          logger.error("Failed to parse WebSocket message:", error);
        }
      };
    } catch (error) {
      logger.error("Failed to create WebSocket connection:", error);
      setIsConnecting(false);
      isConnectingRef.current = false;
      setConnectionState(ConnectionState.DISCONNECTED);
    }
  }, [url, maxReconnectAttempts, reconnectInterval]);

  const disconnect = useCallback(() => {
    shouldReconnectRef.current = false;

    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.onerror = null;
      wsRef.current.onmessage = null;
      wsRef.current.close();
      wsRef.current = null;
    }

    setIsConnected(false);
    setIsConnecting(false);
    isConnectingRef.current = false;
    setConnectionState(ConnectionState.DISCONNECTED);
  }, []);

  const send = useCallback((message: WsMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    } else {
      logger.warn("WebSocket is not connected. Cannot send message:", message);
    }
  }, []);

  const subscribe = useCallback(
    (channels: string[]) => {
      send({
        type: "subscribe",
        channels,
      });
    },
    [send],
  );

  const unsubscribe = useCallback(
    (channels: string[]) => {
      send({
        type: "unsubscribe",
        channels,
      });
    },
    [send],
  );

  const reconnect = useCallback(() => {
    disconnect();

    shouldReconnectRef.current = true;
    connectionAttemptsRef.current = 0;
    setConnectionAttempts(0);

    setTimeout(() => {
      connect();
    }, 100);
  }, [connect, disconnect]);

  useEffect(() => {
    shouldReconnectRef.current = true;
    connect();

    return () => {
      shouldReconnectRef.current = false;
      disconnect();
    };
  }, [connect, disconnect]);

  // Handle visibility change (background/foreground)
  useEffect(() => {
    const handleVisibilityChange = () => {
      const nextState = document.visibilityState === "hidden" ? "background" : "active";
      const prevState = appStateRef.current;

      if (nextState === prevState) return;

      setAppState(nextState);

      if (nextState === "active" && prevState === "background") {
        logger.debug("App active. Resuming WebSocket reconnection with exponential backoff.");
        if (shouldReconnectRef.current && !isConnected && !isConnectingRef.current) {
          const delay = reconnectInterval * Math.pow(1.5, connectionAttemptsRef.current);
          if (reconnectTimeoutRef.current) {
            clearTimeout(reconnectTimeoutRef.current);
          }
          reconnectTimeoutRef.current = setTimeout(() => {
            connect();
          }, delay);
          connectionAttemptsRef.current += 1;
          setConnectionAttempts(connectionAttemptsRef.current);
        }
      } else if (nextState === "background") {
        logger.debug("App in background. Pausing WebSocket reconnection.");
        if (reconnectTimeoutRef.current) {
          clearTimeout(reconnectTimeoutRef.current);
          reconnectTimeoutRef.current = null;
        }
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => {
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [connect, isConnected, reconnectInterval]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      shouldReconnectRef.current = false;
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
    };
  }, []);

  return {
    isConnected,
    isConnecting,
    lastMessage,
    connectionAttempts,
    send,
    subscribe,
    unsubscribe,
    reconnect,
  };
}
