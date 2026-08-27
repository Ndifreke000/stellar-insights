"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import { logger } from "@/lib/logger";
import type { CorridorPerformanceAlert } from "@/lib/alerts-api";

interface UseCorridorPerformanceAlertsOptions {
  onAlert?: (alert: CorridorPerformanceAlert) => void;
}

interface UseCorridorPerformanceAlertsReturn {
  alerts: CorridorPerformanceAlert[];
  isConnected: boolean;
  clearAlerts: () => void;
  acknowledgeAlert: (index: number) => void;
}

export function useCorridorPerformanceAlerts(
  options: UseCorridorPerformanceAlertsOptions = {},
): UseCorridorPerformanceAlertsReturn {
  const { onAlert } = options;
  const [alerts, setAlerts] = useState<CorridorPerformanceAlert[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const onAlertRef = useRef(onAlert);
  onAlertRef.current = onAlert;

  const connect = useCallback(() => {
    try {
      const wsUrl =
        process.env.NEXT_PUBLIC_WS_URL || "ws://localhost:8080/ws";
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        setIsConnected(true);
        // Subscribe to corridor performance alerts channel
        ws.send(
          JSON.stringify({
            type: "subscribe",
            channels: ["corridor_performance_alerts"],
          }),
        );
        logger.debug("Connected to corridor performance alerts WebSocket");
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          if (
            data.alert_type &&
            data.corridor_key &&
            data.severity
          ) {
            const alert: CorridorPerformanceAlert = {
              config_id: data.config_id || "",
              corridor_key: data.corridor_key,
              alert_type: data.alert_type,
              severity: data.severity,
              message: data.message,
              old_value: data.old_value,
              new_value: data.new_value,
              threshold_value: data.threshold_value,
              timestamp: data.timestamp,
            };
            setAlerts((prev) => [alert, ...prev].slice(0, 50));
            onAlertRef.current?.(alert);
          }
        } catch {
          // Not a JSON message or not an alert
        }
      };

      ws.onclose = () => {
        setIsConnected(false);
        // Reconnect after 5 seconds
        reconnectTimeoutRef.current = setTimeout(connect, 5000);
      };

      ws.onerror = () => {
        ws.close();
      };
    } catch (err) {
      logger.error("Failed to connect to corridor performance alerts:", err);
    }
  }, []);

  useEffect(() => {
    connect();
    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      wsRef.current?.close();
    };
  }, [connect]);

  const clearAlerts = useCallback(() => {
    setAlerts([]);
  }, []);

  const acknowledgeAlert = useCallback((index: number) => {
    setAlerts((prev) => prev.filter((_, i) => i !== index));
  }, []);

  return {
    alerts,
    isConnected,
    clearAlerts,
    acknowledgeAlert,
  };
}
