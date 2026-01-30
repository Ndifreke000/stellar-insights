
import { useEffect, useRef, useCallback, useState } from 'react';
import { WebSocketNotificationPayload } from '@/types/notifications';

interface UseWebSocketOptions {
  url: string;
  onMessage?: (data: WebSocketNotificationPayload) => void;
  onConnect?: () => void;
  onDisconnect?: () => void;
  onError?: (error: Event) => void;
  reconnectAttempts?: number;
  reconnectInterval?: number;
  heartbeatInterval?: number;
}

interface WebSocketState {
  isConnected: boolean;
  isConnecting: boolean;
  error: string | null;
  reconnectCount: number;
}

export const useWebSocket = ({
  url,
  onMessage,
  onConnect,
  onDisconnect,
  onError,
  reconnectAttempts = 5,
  reconnectInterval = 3000,
  heartbeatInterval = 30000,
}: UseWebSocketOptions) => {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const heartbeatTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const reconnectCountRef = useRef(0);
  const isManualCloseRef = useRef(false);
  const isMountedRef = useRef(true);

  const [state, setState] = useState<WebSocketState>({
    isConnected: false,
    isConnecting: false,
    error: null,
    reconnectCount: 0,
  });

  // Don't attempt connection if URL is empty
  const shouldConnect = url && url.trim() !== '';

  // Safe setState that checks if component is still mounted
  const safeSetState = useCallback((updater: (prev: WebSocketState) => WebSocketState) => {
    if (isMountedRef.current) {
      setState(updater);
    }
  }, []);

  const clearTimeouts = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
    if (heartbeatTimeoutRef.current) {
      clearTimeout(heartbeatTimeoutRef.current);
      heartbeatTimeoutRef.current = null;
    }
  }, []);

  const startHeartbeat = useCallback(() => {
    if (heartbeatTimeoutRef.current) {
      clearTimeout(heartbeatTimeoutRef.current);
    }

    heartbeatTimeoutRef.current = setTimeout(() => {
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify({ type: 'ping' }));
        startHeartbeat();
      }
    }, heartbeatInterval);
  }, [heartbeatInterval]);

  const connectRef = useRef<() => void>(() => {});
  const disconnectRef = useRef<() => void>(() => {});

  const connect = useCallback(() => {
    // Don't connect if URL is empty or invalid
    if (!shouldConnect) {
      return;
    }

    if (wsRef.current?.readyState === WebSocket.CONNECTING || 
        wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    safeSetState(prev => ({ ...prev, isConnecting: true, error: null }));

    try {
      wsRef.current = new WebSocket(url);

      wsRef.current.onopen = () => {
        safeSetState(prev => ({ 
          ...prev, 
          isConnected: true, 
          isConnecting: false, 
          error: null,
          reconnectCount: reconnectCountRef.current 
        }));
        reconnectCountRef.current = 0;
        startHeartbeat();
        onConnect?.();
      };

      wsRef.current.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          
          // Handle heartbeat response
          if (data.type === 'pong') {
            return;
          }

          // Handle notification messages
          if (data.type && data.data) {
            onMessage?.(data as WebSocketNotificationPayload);
          }
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };

      wsRef.current.onclose = (event) => {
        safeSetState(prev => ({ 
          ...prev, 
          isConnected: false, 
          isConnecting: false 
        }));
        clearTimeouts();
        onDisconnect?.();

        // Attempt reconnection if not manually closed
        if (!isManualCloseRef.current && reconnectCountRef.current < reconnectAttempts) {
          reconnectCountRef.current++;
          safeSetState(prev => ({ ...prev, reconnectCount: reconnectCountRef.current }));
          
          reconnectTimeoutRef.current = setTimeout(() => {
            connectRef.current?.();
          }, reconnectInterval * Math.pow(1.5, reconnectCountRef.current - 1)); // Exponential backoff
        }
      };

      wsRef.current.onerror = (error) => {
        safeSetState(prev => ({ 
          ...prev, 
          error: 'WebSocket connection failed',
          isConnecting: false 
        }));
        onError?.(error);
      };

    } catch (error) {
      safeSetState(prev => ({ 
        ...prev, 
        error: 'Failed to create WebSocket connection',
        isConnecting: false 
      }));
    }
  }, [url, onMessage, onConnect, onDisconnect, onError, reconnectAttempts, reconnectInterval, startHeartbeat, clearTimeouts, shouldConnect]);

  const disconnect = useCallback(() => {
    isManualCloseRef.current = true;
    clearTimeouts();
    
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    
    safeSetState(prev => ({ 
      ...prev, 
      isConnected: false, 
      isConnecting: false 
    }));
    reconnectCountRef.current = 0;
  }, [clearTimeouts]);

  // Update refs
  connectRef.current = connect;
  disconnectRef.current = disconnect;

  const sendMessage = useCallback((message: any) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
      return true;
    }
    return false;
  }, []);

  useEffect(() => {
    if (shouldConnect) {
      connectRef.current?.();
    }

    return () => {
      disconnectRef.current?.();
    };
  }, [shouldConnect]); // Remove connect and disconnect from dependencies

  // Reset manual close flag when component unmounts and mark as unmounted
  useEffect(() => {
    return () => {
      isManualCloseRef.current = false;
      isMountedRef.current = false;
    };
  }, []);

  return {
    ...state,
    connect,
    disconnect,
    sendMessage,
  };
};

