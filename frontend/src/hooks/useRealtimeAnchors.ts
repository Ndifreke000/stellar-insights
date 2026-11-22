import { useEffect, useState, useCallback, useMemo, useRef } from "react";
import { useWebSocket, type WsMessage } from "./useWebSocket";
import { logger } from "@/lib/logger";

export interface AnchorUpdate {
  anchor_id: string;
  name: string;
  reliability_score: number;
  status: string;
}

export interface UseRealtimeAnchorsOptions {
  anchorIds?: string[];
  onAnchorUpdate?: (update: AnchorUpdate) => void;
}

export interface UseRealtimeAnchorsReturn {
  isConnected: boolean;
  isConnecting: boolean;
  connectionAttempts: number;
  anchorUpdates: Map<string, AnchorUpdate>;
  subscribeToAnchors: (anchorIds: string[]) => void;
  unsubscribeFromAnchors: (anchorIds: string[]) => void;
  reconnect: () => void;
}

export function useRealtimeAnchors(
  options: UseRealtimeAnchorsOptions = {},
): UseRealtimeAnchorsReturn {
  const { anchorIds = [], onAnchorUpdate } = options;

  const [anchorUpdates, setAnchorUpdates] = useState<Map<string, AnchorUpdate>>(
    new Map(),
  );

  const onAnchorUpdateRef = useRef(onAnchorUpdate);
  useEffect(() => {
    onAnchorUpdateRef.current = onAnchorUpdate;
  });

  const stableAnchorIdsStr = useMemo(
    () => Array.from(new Set(anchorIds.filter(Boolean))).sort().join("\0"),
    [anchorIds],
  );

  const channels = useMemo(() => {
    if (!stableAnchorIdsStr) {
      return ["anchors"];
    }
    return stableAnchorIdsStr.split("\0").map((id) => `anchor:${id}`);
  }, [stableAnchorIdsStr]);

  const handleMessage = useCallback((message: WsMessage) => {
    switch (message.type) {
      case "anchor_update": {
        const anchorUpdate = message as unknown as AnchorUpdate;
        if (!anchorUpdate.anchor_id) return;
        setAnchorUpdates((prev) => {
          const newMap = new Map(prev);
          newMap.set(anchorUpdate.anchor_id, anchorUpdate);
          return newMap;
        });
        onAnchorUpdateRef.current?.(anchorUpdate);
        break;
      }

      case "subscription_confirm":
        logger.debug("Anchor subscription confirmed:", message);
        break;

      default:
        break;
    }
  }, []);

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
      logger.debug("Connected to shared anchor realtime stream");
    },
    onClose: () => {
      logger.debug("Disconnected from shared anchor realtime stream");
    },
    onError: (error) => {
      logger.error("Anchor WebSocket error:", error);
    },
  });

  const subscribeToAnchors = useCallback(
    (ids: string[]) => {
      subscribe(ids.map((id) => `anchor:${id}`));
    },
    [subscribe],
  );

  const unsubscribeFromAnchors = useCallback(
    (ids: string[]) => {
      unsubscribe(ids.map((id) => `anchor:${id}`));
    },
    [unsubscribe],
  );

  return {
    isConnected,
    isConnecting,
    connectionAttempts,
    anchorUpdates,
    subscribeToAnchors,
    unsubscribeFromAnchors,
    reconnect,
  };
}
