import {
  useEffect,
  useRef,
  useCallback,
  useSyncExternalStore,
} from "react";
import { useRealtimeClient } from "@/contexts/RealtimeContext";
import {
  ConnectionState,
  type WsMessage,
} from "@/lib/websocket";

export { ConnectionState, type WsMessage };

export interface UseWebSocketOptions {
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  maxReconnectDelay?: number;
  backoffFactor?: number;
  /** Initial channels to acquire on mount and release on unmount. */
  channels?: string[];
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (error: Event) => void;
  onMessage?: (message: WsMessage) => void;
}

export interface UseWebSocketReturn {
  isConnected: boolean;
  isConnecting: boolean;
  connectionState: ConnectionState;
  lastMessage: WsMessage | null;
  connectionAttempts: number;
  send: (message: WsMessage) => void;
  subscribe: (channels: string[]) => void;
  unsubscribe: (channels: string[]) => void;
  reconnect: () => void;
}

/**
 * Subscribe to the shared tab-wide [`RealtimeClient`] via context (Issue #2834).
 *
 * Multiple components invoking `useWebSocket` share a single underlying
 * WebSocket connection with reference-counted channel subscriptions and
 * automatic cleanup on unmount.
 */
export function useWebSocket(
  url?: string,
  options: UseWebSocketOptions = {},
): UseWebSocketReturn {
  const {
    reconnectInterval,
    maxReconnectAttempts,
    maxReconnectDelay,
    backoffFactor,
    channels: initialChannels,
    onOpen,
    onClose,
    onError,
    onMessage,
  } = options;

  const client = useRealtimeClient(url);

  // Propagate caller timing overrides (e.g. fast reconnect intervals in tests).
  useEffect(() => {
    if (
      reconnectInterval !== undefined ||
      maxReconnectAttempts !== undefined ||
      maxReconnectDelay !== undefined ||
      backoffFactor !== undefined
    ) {
      client.configure({
        reconnectInterval,
        maxReconnectAttempts,
        maxReconnectDelay,
        backoffFactor,
      });
    }
  }, [
    client,
    reconnectInterval,
    maxReconnectAttempts,
    maxReconnectDelay,
    backoffFactor,
  ]);

  const snapshot = useSyncExternalStore(
    client.subscribeState,
    client.getSnapshot,
    client.getSnapshot,
  );

  // Track channels owned by THIS hook instance so repeated subscribe calls
  // from the same hook stay idempotent and every channel is released on unmount.
  const ownedChannelsRef = useRef<Set<string>>(new Set());

  const callbacksRef = useRef({ onOpen, onClose, onError, onMessage });
  useEffect(() => {
    callbacksRef.current = { onOpen, onClose, onError, onMessage };
  });

  // Attach to the shared client lifecycle and register event callbacks.
  useEffect(() => {
    const ownedChannels = ownedChannelsRef.current;

    const unsubOpen = client.onOpen(() => callbacksRef.current.onOpen?.());
    const unsubClose = client.onClose(() => callbacksRef.current.onClose?.());
    const unsubError = client.onError((err) =>
      callbacksRef.current.onError?.(err),
    );
    const unsubMessage = client.onAny((msg) =>
      callbacksRef.current.onMessage?.(msg),
    );
    const detach = client.attach();

    // If the shared socket was already open when this hook mounted, notify onOpen.
    if (client.isConnected()) {
      callbacksRef.current.onOpen?.();
    }

    return () => {
      unsubOpen();
      unsubClose();
      unsubError();
      unsubMessage();
      if (ownedChannels.size > 0) {
        const toRelease = Array.from(ownedChannels);
        ownedChannels.clear();
        client.releaseChannels(toRelease);
      }
      detach();
    };
  }, [client]);

  const subscribe = useCallback(
    (channels: string[]) => {
      const newlyOwned: string[] = [];
      for (const raw of channels) {
        const ch = raw.trim();
        if (ch && !ownedChannelsRef.current.has(ch)) {
          ownedChannelsRef.current.add(ch);
          newlyOwned.push(ch);
        }
      }
      if (newlyOwned.length > 0) {
        client.acquireChannels(newlyOwned);
      }
    },
    [client],
  );

  const unsubscribe = useCallback(
    (channels: string[]) => {
      const toRelease: string[] = [];
      for (const raw of channels) {
        const ch = raw.trim();
        if (ch && ownedChannelsRef.current.has(ch)) {
          ownedChannelsRef.current.delete(ch);
          toRelease.push(ch);
        }
      }
      if (toRelease.length > 0) {
        client.releaseChannels(toRelease);
      }
    },
    [client],
  );

  // Declarative channel subscriptions passed via options.channels.
  const initialChannelsKey = initialChannels
    ? Array.from(new Set(initialChannels.filter(Boolean))).sort().join("\0")
    : "";

  useEffect(() => {
    if (!initialChannelsKey) return;
    const channels = initialChannelsKey.split("\0");
    subscribe(channels);
    return () => {
      unsubscribe(channels);
    };
  }, [initialChannelsKey, subscribe, unsubscribe]);

  const send = useCallback(
    (message: WsMessage) => {
      client.send(message);
    },
    [client],
  );

  const reconnect = useCallback(() => {
    client.reconnect();
  }, [client]);

  return {
    isConnected: snapshot.isConnected,
    isConnecting: snapshot.isConnecting,
    connectionState: snapshot.state,
    lastMessage: snapshot.lastMessage,
    connectionAttempts: snapshot.connectionAttempts,
    send,
    subscribe,
    unsubscribe,
    reconnect,
  };
}
