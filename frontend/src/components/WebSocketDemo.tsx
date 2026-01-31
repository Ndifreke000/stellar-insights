'use client';

import { useWebSocket } from '@/hooks/useWebSocket';
import { useState } from 'react';
import type { WebSocketNotificationPayload } from '@/types/notifications';

/**
 * Example component demonstrating WebSocket real-time updates
 *
 * This component shows how to:
 * 1. Connect to the WebSocket server
 * 2. Display connection status
 * 3. Listen for different message types
 * 4. Display received messages
 */
export function WebSocketDemo() {
  const [messages, setMessages] = useState<WebSocketNotificationPayload[]>([]);
  const [snapshots, setSnapshots] = useState<Array<{ id: string; title: string; time: string }>>([]);
  const [corridors, setCorridors] = useState<Array<{ key: string; title: string; time: string }>>([]);
  const [anchors, setAnchors] = useState<Array<{ name: string; title: string; time: string }>>([]);

  // Get WebSocket URL from environment
  const wsUrl = process.env.NEXT_PUBLIC_WS_URL || '';

  // Main WebSocket connection
  const { isConnected, isConnecting, error, reconnectCount, connect, disconnect, sendMessage } = useWebSocket({
    url: wsUrl,
    onMessage: (message) => {
      setMessages((prev) => [message, ...prev].slice(0, 10)); // Keep last 10 messages

      // Route messages to appropriate handlers based on type
      const metadata = message.data?.metadata as Record<string, unknown> | undefined;
      if (message.type === 'new_snapshot') {
        setSnapshots((prev) => [
          {
            id: String(metadata?.snapshot_id ?? 'unknown'),
            title: message.data.title,
            time: new Date().toLocaleTimeString(),
          },
          ...prev,
        ].slice(0, 5));
      } else if (message.type === 'low_liquidity') {
        setCorridors((prev) => [
          {
            key: String(metadata?.corridor_key ?? 'unknown'),
            title: message.data.title,
            time: new Date().toLocaleTimeString(),
          },
          ...prev,
        ].slice(0, 5));
      } else if (message.type === 'payment_failed') {
        setAnchors((prev) => [
          {
            name: String(metadata?.anchor_name ?? 'unknown'),
            title: message.data.title,
            time: new Date().toLocaleTimeString(),
          },
          ...prev,
        ].slice(0, 5));
      }
    },
    onConnect: () => {
      console.log('WebSocket connected!');
    },
    onDisconnect: () => {
      console.log('WebSocket disconnected!');
    },
    onError: (error) => {
      console.error('WebSocket error:', error);
    },
  });

  const handlePing = () => {
    sendMessage({ type: 'ping' });
  };

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <h1 className="text-3xl font-bold mb-6">WebSocket Real-time Updates Demo</h1>

      {/* Connection Status */}
      <div className="mb-6 p-4 bg-gray-100 dark:bg-gray-800 rounded-lg">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h2 className="text-xl font-semibold mb-2">Connection Status</h2>
            <div className="flex items-center gap-2">
              <div
                className={`w-3 h-3 rounded-full ${
                  isConnected ? 'bg-green-500' : isConnecting ? 'bg-yellow-500' : 'bg-red-500'
                }`}
              />
              <span>{isConnected ? 'Connected' : isConnecting ? 'Connecting...' : 'Disconnected'}</span>
            </div>
            {error && (
              <p className="text-sm text-red-600 dark:text-red-400 mt-1">
                Error: {error}
              </p>
            )}
            {reconnectCount > 0 && (
              <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                Reconnect attempts: {reconnectCount}
              </p>
            )}
          </div>
          <div className="flex gap-2">
            {!isConnected ? (
              <button
                onClick={connect}
                disabled={isConnecting}
                className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50"
              >
                {isConnecting ? 'Connecting...' : 'Connect'}
              </button>
            ) : (
              <>
                <button
                  onClick={handlePing}
                  className="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600"
                >
                  Send Ping
                </button>
                <button
                  onClick={disconnect}
                  className="px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600"
                >
                  Disconnect
                </button>
              </>
            )}
          </div>
        </div>
      </div>

      {/* Real-time Updates Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        {/* Snapshot Updates */}
        <div className="p-4 bg-white dark:bg-gray-900 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-3">Snapshot Updates</h3>
          {snapshots.length === 0 ? (
            <p className="text-gray-500 text-sm">No snapshots yet</p>
          ) : (
            <ul className="space-y-2">
              {snapshots.map((snapshot, i) => (
                <li key={i} className="text-sm border-l-2 border-blue-500 pl-2">
                  <div className="font-medium">{snapshot.title}</div>
                  <div className="text-gray-600 dark:text-gray-400 text-xs">
                    ID: {snapshot.id} - {snapshot.time}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>

        {/* Corridor Updates */}
        <div className="p-4 bg-white dark:bg-gray-900 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-3">Corridor Updates</h3>
          {corridors.length === 0 ? (
            <p className="text-gray-500 text-sm">No corridors yet</p>
          ) : (
            <ul className="space-y-2">
              {corridors.map((corridor, i) => (
                <li key={i} className="text-sm border-l-2 border-green-500 pl-2">
                  <div className="font-medium">{corridor.key}</div>
                  <div className="text-gray-600 dark:text-gray-400 text-xs">
                    {corridor.title} - {corridor.time}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>

        {/* Anchor Updates */}
        <div className="p-4 bg-white dark:bg-gray-900 rounded-lg shadow">
          <h3 className="text-lg font-semibold mb-3">Anchor Updates</h3>
          {anchors.length === 0 ? (
            <p className="text-gray-500 text-sm">No anchors yet</p>
          ) : (
            <ul className="space-y-2">
              {anchors.map((anchor, i) => (
                <li key={i} className="text-sm border-l-2 border-purple-500 pl-2">
                  <div className="font-medium">{anchor.name}</div>
                  <div className="text-gray-600 dark:text-gray-400 text-xs">
                    {anchor.title} - {anchor.time}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      {/* All Messages Log */}
      <div className="p-4 bg-white dark:bg-gray-900 rounded-lg shadow">
        <h3 className="text-lg font-semibold mb-3">Recent Messages</h3>
        {messages.length === 0 ? (
          <p className="text-gray-500 text-sm">No messages yet</p>
        ) : (
          <div className="space-y-2">
            {messages.map((message, i) => (
              <div
                key={i}
                className="p-3 bg-gray-50 dark:bg-gray-800 rounded text-sm font-mono"
              >
                <div className="font-semibold text-blue-600 dark:text-blue-400 mb-1">
                  {message.type}
                </div>
                <pre className="text-xs overflow-auto">
                  {JSON.stringify(message, null, 2)}
                </pre>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
