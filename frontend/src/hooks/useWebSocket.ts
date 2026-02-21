import { useEffect, useRef, useState, useCallback } from 'react';

export interface WsMessage {
  type: string;
  [key: string]: any;
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
  options: UseWebSocketOptions = {}
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

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const shouldReconnectRef = useRef(true);

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    setIsConnecting(true);
    
    try {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        console.log('WebSocket connected');
        setIsConnected(true);
        setIsConnecting(false);
        setConnectionAttempts(0);
        onOpen?.();
      };

      ws.onclose = () => {
        console.log('WebSocket disconnected');
        setIsConnected(false);
        setIsConnecting(false);
        onClose?.();

        // Attempt to reconnect if enabled and under max attempts
        if (
          shouldReconnectRef.current &&
          connectionAttempts < maxReconnectAttempts
        ) {
          setConnectionAttempts(prev => prev + 1);
          reconnectTimeoutRef.current = setTimeout(() => {
            connect();
          }, reconnectInterval * Math.pow(1.5, connectionAttempts)); // Exponential backoff
        }
      };

      ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        setIsConnecting(false);
        onError?.(error);
      };

      ws.onmessage = (event) => {
        try {
          const message: WsMessage = JSON.parse(event.data);
          setLastMessage(message);
          onMessage?.(message);
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };
    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
      setIsConnecting(false);
    }
  }, [url, connectionAttempts, maxReconnectAttempts, reconnectInterval, onOpen, onClose, onError, onMessage]);

  const disconnect = useCallback(() => {
    shouldReconnectRef.current = false;
    
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
  }, []);

  const send = useCallback((message: WsMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    } else {
      console.warn('WebSocket is not connected. Cannot send message:', message);
    }
  }, []);

  const subscribe = useCallback((channels: string[]) => {
    send({
      type: 'subscribe',
      channels,
    });
  }, [send]);

  const unsubscribe = useCallback((channels: string[]) => {
    send({
      type: 'unsubscribe',
      channels,
    });
  }, [send]);

  const reconnect = useCallback(() => {
    disconnect();
    shouldReconnectRef.current = true;
    setConnectionAttempts(0);
    setTimeout(connect, 100);
  }, [connect, disconnect]);

  useEffect(() => {
    connect();

    return () => {
      disconnect();
    };
  }, [connect, disconnect]);

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