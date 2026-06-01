import { useEffect, useState, useCallback } from 'react';
import { useWebSocket, WsMessage } from './useWebSocket';

export interface CorridorUpdate {
  corridor_key: string;
  asset_a_code: string;
  asset_a_issuer: string;
  asset_b_code: string;
  asset_b_issuer: string;
  success_rate?: number;
  health_score?: number;
  last_updated?: string;
}

export interface HealthAlert {
  corridor_id: string;
  severity: 'info' | 'warning' | 'error' | 'critical';
  message: string;
  timestamp: string;
}

export interface NewPayment {
  corridor_id: string;
  amount: number;
  successful: boolean;
  timestamp: string;
}

export interface UseRealtimeCorridorsOptions {
  corridorKeys?: string[];
  enablePaymentStream?: boolean;
  onCorridorUpdate?: (update: CorridorUpdate) => void;
  onHealthAlert?: (alert: HealthAlert) => void;
  onNewPayment?: (payment: NewPayment) => void;
}

export interface UseRealtimeCorridorsReturn {
  isConnected: boolean;
  isConnecting: boolean;
  connectionAttempts: number;
  corridorUpdates: Map<string, CorridorUpdate>;
  healthAlerts: HealthAlert[];
  recentPayments: NewPayment[];
  subscribeToCorridors: (corridorKeys: string[]) => void;
  unsubscribeFromCorridors: (corridorKeys: string[]) => void;
  clearHealthAlerts: () => void;
  reconnect: () => void;
}

export function useRealtimeCorridors(
  options: UseRealtimeCorridorsOptions = {}
): UseRealtimeCorridorsReturn {
  const {
    corridorKeys = [],
    enablePaymentStream = false,
    onCorridorUpdate,
    onHealthAlert,
    onNewPayment,
  } = options;

  const [corridorUpdates, setCorridorUpdates] = useState<Map<string, CorridorUpdate>>(new Map());
  const [healthAlerts, setHealthAlerts] = useState<HealthAlert[]>([]);
  const [recentPayments, setRecentPayments] = useState<NewPayment[]>([]);

  // Get WebSocket URL from environment or default
  const wsUrl = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:8080/ws';

  const handleMessage = useCallback((message: WsMessage) => {
    switch (message.type) {
      case 'corridor_update':
        const corridorUpdate = message as CorridorUpdate;
        setCorridorUpdates(prev => {
          const newMap = new Map(prev);
          newMap.set(corridorUpdate.corridor_key, corridorUpdate);
          return newMap;
        });
        onCorridorUpdate?.(corridorUpdate);
        break;

      case 'health_alert':
        const healthAlert = message as HealthAlert;
        setHealthAlerts(prev => [healthAlert, ...prev].slice(0, 50)); // Keep last 50 alerts
        onHealthAlert?.(healthAlert);
        break;

      case 'new_payment':
        if (enablePaymentStream) {
          const payment = message as NewPayment;
          setRecentPayments(prev => [payment, ...prev].slice(0, 100)); // Keep last 100 payments
          onNewPayment?.(payment);
        }
        break;

      case 'subscription_confirm':
        console.log('Subscription confirmed:', message);
        break;

      case 'ping':
        // Handle ping/pong automatically
        break;

      default:
        console.log('Unhandled WebSocket message:', message);
    }
  }, [enablePaymentStream, onCorridorUpdate, onHealthAlert, onNewPayment]);

  const {
    isConnected,
    isConnecting,
    connectionAttempts,
    subscribe,
    unsubscribe,
    reconnect,
  } = useWebSocket(wsUrl, {
    onMessage: handleMessage,
    onOpen: () => {
      console.log('Connected to corridor WebSocket');
      // Re-subscribe to corridors on reconnection
      if (corridorKeys.length > 0) {
        subscribeToCorridors(corridorKeys);
      }
    },
    onClose: () => {
      console.log('Disconnected from corridor WebSocket');
    },
    onError: (error) => {
      console.error('Corridor WebSocket error:', error);
    },
  });

  const subscribeToCorridors = useCallback((keys: string[]) => {
    const channels = keys.map(key => `corridor:${key}`);
    if (enablePaymentStream) {
      channels.push(...keys.map(key => `payments:${key}`));
    }
    subscribe(channels);
  }, [subscribe, enablePaymentStream]);

  const unsubscribeFromCorridors = useCallback((keys: string[]) => {
    const channels = keys.map(key => `corridor:${key}`);
    if (enablePaymentStream) {
      channels.push(...keys.map(key => `payments:${key}`));
    }
    unsubscribe(channels);
  }, [unsubscribe, enablePaymentStream]);

  const clearHealthAlerts = useCallback(() => {
    setHealthAlerts([]);
  }, []);

  // Subscribe to initial corridors when connected
  useEffect(() => {
    if (isConnected && corridorKeys.length > 0) {
      subscribeToCorridors(corridorKeys);
    }
  }, [isConnected, corridorKeys, subscribeToCorridors]);

  return {
    isConnected,
    isConnecting,
    connectionAttempts,
    corridorUpdates,
    healthAlerts,
    recentPayments,
    subscribeToCorridors,
    unsubscribeFromCorridors,
    clearHealthAlerts,
    reconnect,
  };
}