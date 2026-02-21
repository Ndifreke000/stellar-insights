import { useEffect, useState, useCallback } from 'react';
import { useWebSocket, WsMessage } from './useWebSocket';

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
  options: UseRealtimeAnchorsOptions = {}
): UseRealtimeAnchorsReturn {
  const {
    anchorIds = [],
    onAnchorUpdate,
  } = options;

  const [anchorUpdates, setAnchorUpdates] = useState<Map<string, AnchorUpdate>>(new Map());

  // Get WebSocket URL from environment or default
  const wsUrl = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:8080/ws';

  const handleMessage = useCallback((message: WsMessage) => {
    switch (message.type) {
      case 'anchor_update':
        const anchorUpdate = message as AnchorUpdate;
        setAnchorUpdates(prev => {
          const newMap = new Map(prev);
          newMap.set(anchorUpdate.anchor_id, anchorUpdate);
          return newMap;
        });
        onAnchorUpdate?.(anchorUpdate);
        break;

      case 'subscription_confirm':
        console.log('Anchor subscription confirmed:', message);
        break;

      case 'ping':
        // Handle ping/pong automatically
        break;

      default:
        // Ignore other message types
        break;
    }
  }, [onAnchorUpdate]);

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
      console.log('Connected to anchor WebSocket');
      // Re-subscribe to anchors on reconnection
      if (anchorIds.length > 0) {
        subscribeToAnchors(anchorIds);
      }
    },
    onClose: () => {
      console.log('Disconnected from anchor WebSocket');
    },
    onError: (error) => {
      console.error('Anchor WebSocket error:', error);
    },
  });

  const subscribeToAnchors = useCallback((ids: string[]) => {
    const channels = ids.map(id => `anchor:${id}`);
    subscribe(channels);
  }, [subscribe]);

  const unsubscribeFromAnchors = useCallback((ids: string[]) => {
    const channels = ids.map(id => `anchor:${id}`);
    unsubscribe(channels);
  }, [unsubscribe]);

  // Subscribe to initial anchors when connected
  useEffect(() => {
    if (isConnected && anchorIds.length > 0) {
      subscribeToAnchors(anchorIds);
    }
  }, [isConnected, anchorIds, subscribeToAnchors]);

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