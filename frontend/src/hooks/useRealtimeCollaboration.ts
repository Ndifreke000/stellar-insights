import { useState, useCallback, useRef, useEffect } from 'react';

interface CollaborativeUser {
  id: string;
  name: string;
  color: string;
  lastActive: Date;
  isOnline: boolean;
}

interface CollaborativeMessage {
  id: string;
  userId: string;
  userName: string;
  content: string;
  timestamp: Date;
  userColor: string;
}

interface UseRealtimeCollaborationOptions {
  sessionId: string;
  userId: string;
  userName: string;
  autoConnect?: boolean;
}

export const useRealtimeCollaboration = (options: UseRealtimeCollaborationOptions) => {
  const { sessionId, userId, userName, autoConnect = true } = options;

  const [messages, setMessages] = useState<CollaborativeMessage[]>([]);
  const [activeUsers, setActiveUsers] = useState<CollaborativeUser[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState<'connecting' | 'connected' | 'disconnected'>(
    () => (autoConnect ? 'connecting' : 'disconnected'),
  );
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);

  // Opens the WebSocket and wires up its listeners. Does not itself set the
  // 'connecting' status, so it can be called from the mount effect without
  // triggering a synchronous setState from within the effect body.
  const openConnection = useCallback(() => {
    try {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${protocol}//${window.location.host}/api/collaboration/${sessionId}`;

      wsRef.current = new WebSocket(wsUrl);

      wsRef.current.onopen = () => {
        setIsConnected(true);
        setConnectionStatus('connected');
        setError(null);
      };

      wsRef.current.onerror = () => {
        setError('Connection error');
        setConnectionStatus('disconnected');
        setIsConnected(false);
      };

      wsRef.current.onclose = () => {
        setConnectionStatus('disconnected');
        setIsConnected(false);
      };
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Connection failed';
      setError(errorMsg);
      setConnectionStatus('disconnected');
    }
  }, [sessionId]);

  const connect = useCallback(() => {
    setConnectionStatus('connecting');
    openConnection();
  }, [openConnection]);

  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    setIsConnected(false);
    setConnectionStatus('disconnected');
  }, []);

  const sendMessage = useCallback((content: string) => {
    if (!isConnected || !wsRef.current) {
      setError('Not connected');
      return false;
    }

    try {
      wsRef.current.send(JSON.stringify({
        type: 'message',
        content,
        userId,
        userName,
      }));
      return true;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to send message';
      setError(errorMsg);
      return false;
    }
  }, [isConnected, userId, userName]);

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

  const removeUser = useCallback((userId: string) => {
    setActiveUsers((prev) => prev.filter((u) => u.id !== userId));
  }, []);

  const updateUserStatus = useCallback((userId: string, isOnline: boolean) => {
    setActiveUsers((prev) =>
      prev.map((u) =>
        u.id === userId ? { ...u, isOnline, lastActive: new Date() } : u
      )
    );
  }, []);

  useEffect(() => {
    if (autoConnect) {
      openConnection();
    }

    return () => {
      disconnect();
    };
  }, [autoConnect, openConnection, disconnect]);

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
