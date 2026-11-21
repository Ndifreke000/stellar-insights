import { useState, useCallback, useMemo } from "react";
import { useWebSocket, type WsMessage } from "./useWebSocket";

export interface CollaborativeUser {
  id: string;
  name: string;
  color: string;
  lastActive: Date;
  isOnline: boolean;
}

export interface CollaborativeMessage {
  id: string;
  userId: string;
  userName: string;
  content: string;
  timestamp: Date;
  userColor: string;
}

export interface UseRealtimeCollaborationOptions {
  sessionId: string;
  userId: string;
  userName: string;
  autoConnect?: boolean;
}

/**
 * Collaborative session hook multiplexed onto the shared tab-wide WebSocket
 * client via the ref-counted `collaboration:<sessionId>` channel (Issue #2834).
 */
export const useRealtimeCollaboration = (
  options: UseRealtimeCollaborationOptions,
) => {
  const { sessionId, userId, userName, autoConnect = true } = options;

  const [messages, setMessages] = useState<CollaborativeMessage[]>([]);
  const [activeUsers, setActiveUsers] = useState<CollaborativeUser[]>([]);
  const [manualJoined, setManualJoined] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  const joined = manualJoined ?? autoConnect;

  const sessionChannel = useMemo(
    () => (sessionId ? `collaboration:${sessionId}` : ""),
    [sessionId],
  );

  const channels = useMemo(
    () => (joined && sessionChannel ? [sessionChannel] : []),
    [joined, sessionChannel],
  );

  const handleMessage = useCallback(
    (msg: WsMessage) => {
      if (msg.sessionId && msg.sessionId !== sessionId) {
        return;
      }
      if (
        msg.type === "collaboration_message" ||
        (msg.type === "message" && msg.channel === sessionChannel)
      ) {
        const incoming: CollaborativeMessage = {
          id:
            typeof msg.id === "string"
              ? msg.id
              : `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
          userId: String(msg.userId ?? ""),
          userName: String(msg.userName ?? "Anonymous"),
          content: String(msg.content ?? ""),
          timestamp: msg.timestamp ? new Date(String(msg.timestamp)) : new Date(),
          userColor: String(msg.userColor ?? "#6366f1"),
        };
        setMessages((prev) => [...prev, incoming]);
      } else if (msg.type === "collaboration_presence" && Array.isArray(msg.users)) {
        const users = (msg.users as Array<Record<string, unknown>>).map((u) => ({
          id: String(u.id ?? ""),
          name: String(u.name ?? ""),
          color: String(u.color ?? "#6366f1"),
          lastActive: u.lastActive ? new Date(String(u.lastActive)) : new Date(),
          isOnline: Boolean(u.isOnline ?? true),
        }));
        setActiveUsers(users);
      }
    },
    [sessionId, sessionChannel],
  );

  const {
    isConnected: socketConnected,
    isConnecting: socketConnecting,
    send,
    subscribe,
    unsubscribe,
  } = useWebSocket(undefined, {
    channels,
    onMessage: handleMessage,
    onOpen: () => {
      setError(null);
    },
    onError: () => {
      setError("Connection error");
    },
  });

  const isConnected = joined && socketConnected;
  const connectionStatus: "connecting" | "connected" | "disconnected" = !joined
    ? "disconnected"
    : socketConnected
      ? "connected"
      : socketConnecting
        ? "connecting"
        : "disconnected";

  const connect = useCallback(() => {
    setError(null);
    setManualJoined(true);
    if (sessionChannel) {
      subscribe([sessionChannel]);
    }
  }, [sessionChannel, subscribe]);

  const disconnect = useCallback(() => {
    setManualJoined(false);
    if (sessionChannel) {
      unsubscribe([sessionChannel]);
    }
  }, [sessionChannel, unsubscribe]);

  const sendMessage = useCallback(
    (content: string) => {
      if (!isConnected) {
        setError("Not connected");
        return false;
      }

      try {
        send({
          type: "collaboration_message",
          channel: sessionChannel,
          sessionId,
          content,
          userId,
          userName,
          timestamp: new Date().toISOString(),
        });
        return true;
      } catch (err) {
        const errorMsg =
          err instanceof Error ? err.message : "Failed to send message";
        setError(errorMsg);
        return false;
      }
    },
    [isConnected, send, sessionChannel, sessionId, userId, userName],
  );

  const addMessage = useCallback((message: CollaborativeMessage) => {
    setMessages((prev) => [...prev, message]);
  }, []);

  const clearMessages = useCallback(() => {
    setMessages([]);
  }, []);

  const addUser = useCallback((user: CollaborativeUser) => {
    setActiveUsers((prev) => {
      const exists = prev.some((u) => u.id === user.id);
      return exists ? prev : [...prev, user];
    });
  }, []);

  const removeUser = useCallback((targetUserId: string) => {
    setActiveUsers((prev) => prev.filter((u) => u.id !== targetUserId));
  }, []);

  const updateUserStatus = useCallback(
    (targetUserId: string, isOnline: boolean) => {
      setActiveUsers((prev) =>
        prev.map((u) =>
          u.id === targetUserId
            ? { ...u, isOnline, lastActive: new Date() }
            : u,
        ),
      );
    },
    [],
  );

  return {
    messages,
    activeUsers,
    isConnected,
    connectionStatus,
    error,
    connect,
    disconnect,
    sendMessage,
    addMessage,
    clearMessages,
    addUser,
    removeUser,
    updateUserStatus,
  };
};

export default useRealtimeCollaboration;
