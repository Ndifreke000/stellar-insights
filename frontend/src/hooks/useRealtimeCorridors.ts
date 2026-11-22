import { useEffect, useState, useCallback, useMemo, useRef } from "react";
import { useWebSocket, type WsMessage } from "./useWebSocket";
import { logger } from "@/lib/logger";

export interface CorridorUpdate {
  corridor_key: string;
  asset_a_code: string;
  asset_a_issuer: string;
  asset_b_code: string;
  asset_b_issuer: string;
  success_rate?: number;
  health_score?: number;
  liquidity_depth_usd?: number;
  liquidity_volume_24h_usd?: number;
  average_latency_ms?: number;
  total_transactions?: number;
  last_updated?: string;
}

export interface HealthAlert {
  corridor_id: string;
  severity: "info" | "warning" | "error" | "critical";
  message: string;
  timestamp: string;
}

export interface NewPayment {
  id?: string;
  transaction_hash?: string;
  corridor_id: string;
  amount: number;
  asset_code?: string;
  source_account?: string;
  destination_account?: string;
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
  options: UseRealtimeCorridorsOptions = {},
): UseRealtimeCorridorsReturn {
  const {
    corridorKeys = [],
    enablePaymentStream = false,
    onCorridorUpdate,
    onHealthAlert,
    onNewPayment,
  } = options;

  const [corridorUpdates, setCorridorUpdates] = useState<
    Map<string, CorridorUpdate>
  >(new Map());
  const [healthAlerts, setHealthAlerts] = useState<HealthAlert[]>([]);
  const [recentPayments, setRecentPayments] = useState<NewPayment[]>([]);

  const callbacksRef = useRef({ onCorridorUpdate, onHealthAlert, onNewPayment });
  useEffect(() => {
    callbacksRef.current = { onCorridorUpdate, onHealthAlert, onNewPayment };
  });

  // Stabilize corridor keys so inline array literals (e.g. `[pair]`) do not
  // churn subscriptions on every render.
  const stableCorridorKeysStr = useMemo(
    () => Array.from(new Set(corridorKeys.filter(Boolean))).sort().join("\0"),
    [corridorKeys],
  );

  const channels = useMemo(() => {
    if (!stableCorridorKeysStr) {
      return enablePaymentStream ? ["corridors", "payments"] : ["corridors"];
    }
    const keys = stableCorridorKeysStr.split("\0");
    const list = keys.map((key) => `corridor:${key}`);
    if (enablePaymentStream) {
      list.push(...keys.map((key) => `payments:${key}`));
    }
    return list;
  }, [stableCorridorKeysStr, enablePaymentStream]);

  const handleMessage = useCallback(
    (message: WsMessage) => {
      switch (message.type) {
        case "corridor_update": {
          const corridorUpdate = message as unknown as CorridorUpdate;
          if (!corridorUpdate.corridor_key) return;
          setCorridorUpdates((prev) => {
            const newMap = new Map(prev);
            newMap.set(corridorUpdate.corridor_key, corridorUpdate);
            return newMap;
          });
          callbacksRef.current.onCorridorUpdate?.(corridorUpdate);
          break;
        }

        case "health_alert": {
          const healthAlert = message as unknown as HealthAlert;
          setHealthAlerts((prev) => [healthAlert, ...prev].slice(0, 50));
          callbacksRef.current.onHealthAlert?.(healthAlert);
          break;
        }

        case "new_payment": {
          if (enablePaymentStream) {
            const payment = message as unknown as NewPayment;
            setRecentPayments((prev) => [payment, ...prev].slice(0, 100));
            callbacksRef.current.onNewPayment?.(payment);
          }
          break;
        }

        case "subscription_confirm":
          logger.debug("Corridor subscription confirmed:", message);
          break;

        default:
          break;
      }
    },
    [enablePaymentStream],
  );

  // Shares the single tab WebSocket client via RealtimeContext.
  const {
    isConnected,
    isConnecting,
    connectionAttempts,
    subscribe,
    unsubscribe,
    reconnect,
  } = useWebSocket(undefined, {
    channels,
    onMessage: handleMessage,
    onOpen: () => {
      logger.debug("Connected to shared corridor realtime stream");
    },
    onClose: () => {
      logger.debug("Disconnected from shared corridor realtime stream");
    },
    onError: (error) => {
      logger.error("Corridor WebSocket error:", error);
    },
  });

  const subscribeToCorridors = useCallback(
    (keys: string[]) => {
      const nextChannels = keys.map((key) => `corridor:${key}`);
      if (enablePaymentStream) {
        nextChannels.push(...keys.map((key) => `payments:${key}`));
      }
      subscribe(nextChannels);
    },
    [subscribe, enablePaymentStream],
  );

  const unsubscribeFromCorridors = useCallback(
    (keys: string[]) => {
      const prevChannels = keys.map((key) => `corridor:${key}`);
      if (enablePaymentStream) {
        prevChannels.push(...keys.map((key) => `payments:${key}`));
      }
      unsubscribe(prevChannels);
    },
    [unsubscribe, enablePaymentStream],
  );

  const clearHealthAlerts = useCallback(() => {
    setHealthAlerts([]);
  }, []);

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
