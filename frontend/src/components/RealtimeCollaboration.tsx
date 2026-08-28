'use client';

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { Users, Send, Trash2, Copy } from 'lucide-react';

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
  version?: number;
}

interface ConflictInfo {
  rejectedContent: string;
  serverVersion: number;
  localVersion: number;
}

interface RealtimeCollaborationProps {
  sessionId: string;
  userId: string;
  userName: string;
  onMessageReceived?: (message: CollaborativeMessage) => void;
  onUserJoined?: (user: CollaborativeUser) => void;
  onUserLeft?: (userId: string) => void;
  maxMessages?: number;
}

const COLORS = ['#FF6B6B', '#4ECDC4', '#45B7D1', '#FFA07A', '#98D8C8', '#F7DC6F', '#BB8FCE', '#85C1E2'];

export const RealtimeCollaboration: React.FC<RealtimeCollaborationProps> = ({
  sessionId,
  userId,
  userName,
  onMessageReceived,
  onUserJoined,
  onUserLeft,
  maxMessages = 100,
}) => {
  const [messages, setMessages] = useState<CollaborativeMessage[]>([]);
  const [activeUsers, setActiveUsers] = useState<CollaborativeUser[]>([]);
  const [inputValue, setInputValue] = useState('');
  const [isConnected, setIsConnected] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState<'connecting' | 'connected' | 'disconnected'>('disconnected');
  const [conflict, setConflict] = useState<ConflictInfo | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const versionRef = useRef<number>(0);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const connectWebSocket = useCallback(() => {
    try {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${protocol}//${window.location.host}/api/collaboration/${sessionId}`;

      wsRef.current = new WebSocket(wsUrl);

      wsRef.current.onopen = () => {
        setIsConnected(true);
        setConnectionStatus('connected');
        wsRef.current?.send(JSON.stringify({
          type: 'join',
          userId,
          userName,
          color: COLORS[Math.floor(Math.random() * COLORS.length)],
        }));
      };

      wsRef.current.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);

          switch (data.type) {
            case 'message': {
              if (data.version != null) {
                versionRef.current = data.version;
              }
              const newMessage: CollaborativeMessage = {
                id: data.id,
                userId: data.userId,
                userName: data.userName,
                content: data.content,
                timestamp: new Date(data.timestamp),
                userColor: data.userColor,
                version: data.version,
              };
              setMessages((prev) => {
                const updated = [...prev, newMessage];
                return updated.length > maxMessages ? updated.slice(-maxMessages) : updated;
              });
              onMessageReceived?.(newMessage);
              break;
            }

            case 'conflict': {
              if (data.version != null) {
                versionRef.current = data.version;
              }
              setConflict({
                rejectedContent: data.rejectedContent ?? '',
                serverVersion: data.version ?? 0,
                localVersion: data.localVersion ?? 0,
              });
              break;
            }

            case 'user_joined': {
              const joinedUser: CollaborativeUser = {
                id: data.userId,
                name: data.userName,
                color: data.color,
                lastActive: new Date(),
                isOnline: true,
              };
              setActiveUsers((prev) => [...prev, joinedUser]);
              onUserJoined?.(joinedUser);
              break;
            }

            case 'user_left':
              setActiveUsers((prev) => prev.filter((u) => u.id !== data.userId));
              onUserLeft?.(data.userId);
              break;

            case 'users_list':
              setActiveUsers(data.users.map((u: { id: string; name: string; color: string; lastActive: string; isOnline: boolean }) => ({
                id: u.id,
                name: u.name,
                color: u.color,
                lastActive: new Date(u.lastActive),
                isOnline: u.isOnline,
              })));
              break;
          }
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };

      wsRef.current.onerror = () => {
        setConnectionStatus('disconnected');
        setIsConnected(false);
      };

      wsRef.current.onclose = () => {
        setConnectionStatus('disconnected');
        setIsConnected(false);
      };
    } catch (error) {
      console.error('WebSocket connection failed:', error);
      setConnectionStatus('disconnected');
    }
  }, [sessionId, userId, userName, onMessageReceived, onUserJoined, onUserLeft, maxMessages]);

  useEffect(() => {
    connectWebSocket();

    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [connectWebSocket]);

  const sendMessage = useCallback(() => {
    if (!inputValue.trim() || !isConnected || !wsRef.current) return;

    setConflict(null);
    wsRef.current.send(JSON.stringify({
      type: 'message',
      content: inputValue,
      userId,
      userName,
      version: versionRef.current,
    }));

    setInputValue('');
  }, [inputValue, isConnected, userId, userName]);

  const clearMessages = useCallback(() => {
    setMessages([]);
  }, []);

  const copyMessage = useCallback((content: string) => {
    navigator.clipboard.writeText(content);
  }, []);

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  return (
    <div className="flex flex-col h-full bg-white rounded-lg shadow-lg overflow-hidden" role="region" aria-label="Real-time Collaboration">
      {/* Header */}
      <div className="bg-gradient-to-r from-blue-500 to-blue-600 text-white p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Users size={20} />
            <h2 className="text-lg font-bold">Real-time Collaboration</h2>
          </div>
          <div className="flex items-center gap-2">
            <div className={`w-3 h-3 rounded-full ${connectionStatus === 'connected' ? 'bg-green-400' : 'bg-red-400'}`} />
            <span className="text-sm capitalize">{connectionStatus}</span>
          </div>
        </div>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Messages Area */}
        <div className="flex-1 flex flex-col">
          {/* Messages List */}
          <div className="flex-1 overflow-y-auto p-4 space-y-3">
            {messages.length === 0 ? (
              <div className="flex items-center justify-center h-full text-gray-400">
                <p>No messages yet. Start collaborating!</p>
              </div>
            ) : (
              messages.map((msg) => (
                <div
                  key={msg.id}
                  className="flex gap-3 group"
                  role="article"
                  aria-label={`Message from ${msg.userName}`}
                >
                  <div
                    className="w-8 h-8 rounded-full flex items-center justify-center text-white text-xs font-bold flex-shrink-0"
                    style={{ backgroundColor: msg.userColor }}
                  >
                    {msg.userName.charAt(0).toUpperCase()}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-gray-800">{msg.userName}</span>
                      <span className="text-xs text-gray-500">
                        {msg.timestamp.toLocaleTimeString()}
                      </span>
                    </div>
                    <p className="text-gray-700 break-words">{msg.content}</p>
                  </div>
                  <button
                    onClick={() => copyMessage(msg.content)}
                    className="opacity-0 group-hover:opacity-100 transition-opacity p-1 hover:bg-gray-100 rounded"
                    aria-label="Copy message"
                  >
                    <Copy size={16} className="text-gray-500" />
                  </button>
                </div>
              ))
            )}
            <div ref={messagesEndRef} />
          </div>

          {/* Conflict Banner */}
          {conflict && (
            <div className="border-t border-amber-300 bg-amber-50 px-4 py-3 flex items-start gap-2" role="alert" aria-label="Write conflict detected">
              <span className="text-amber-600 font-medium text-sm shrink-0">Conflict:</span>
              <span className="text-amber-800 text-sm flex-1">
                Your message was rejected because another change arrived first. Please retry.
              </span>
              <button
                onClick={() => setConflict(null)}
                className="text-amber-600 hover:text-amber-800 text-sm font-medium shrink-0"
                aria-label="Dismiss conflict"
              >
                Dismiss
              </button>
            </div>
          )}

          {/* Input Area */}
          <div className="border-t p-4 bg-gray-50">
            <div className="flex gap-2">
              <textarea
                value={inputValue}
                onChange={(e) => setInputValue(e.target.value)}
                onKeyPress={handleKeyPress}
                placeholder="Type your message... (Shift+Enter for new line)"
                className="flex-1 p-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                rows={2}
                disabled={!isConnected}
                aria-label="Message input"
              />
              <button
                onClick={sendMessage}
                disabled={!isConnected || !inputValue.trim()}
                className="bg-blue-500 hover:bg-blue-600 disabled:bg-gray-300 text-white p-2 rounded-lg transition-colors flex items-center gap-2"
                aria-label="Send message"
              >
                <Send size={18} />
              </button>
            </div>
          </div>
        </div>

        {/* Sidebar - Active Users */}
        <div className="w-48 border-l bg-gray-50 flex flex-col">
          <div className="p-4 border-b">
            <h3 className="font-semibold text-gray-800 flex items-center gap-2">
              <Users size={16} />
              Active Users ({activeUsers.length})
            </h3>
          </div>
          <div className="flex-1 overflow-y-auto p-3 space-y-2">
            {activeUsers.map((user) => (
              <div
                key={user.id}
                className="flex items-center gap-2 p-2 rounded-lg hover:bg-gray-200 transition-colors"
                role="listitem"
              >
                <div
                  className="w-6 h-6 rounded-full flex items-center justify-center text-white text-xs font-bold"
                  style={{ backgroundColor: user.color }}
                >
                  {user.name.charAt(0).toUpperCase()}
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-gray-800 truncate">{user.name}</p>
                  <p className="text-xs text-gray-500">
                    {user.isOnline ? 'Online' : 'Away'}
                  </p>
                </div>
              </div>
            ))}
          </div>
          <div className="p-4 border-t">
            <button
              onClick={clearMessages}
              className="w-full flex items-center justify-center gap-2 px-3 py-2 text-sm text-red-600 hover:bg-red-50 rounded-lg transition-colors"
              aria-label="Clear all messages"
            >
              <Trash2 size={16} />
              Clear
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default RealtimeCollaboration;
