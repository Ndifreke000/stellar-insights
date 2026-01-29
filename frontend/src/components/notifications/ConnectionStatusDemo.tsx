'use client';

import React, { useState } from 'react';
import { ConnectionStatus } from './ConnectionStatus';
import { useNotifications } from '@/contexts/NotificationContext';

export const ConnectionStatusDemo: React.FC = () => {
  const { showToast } = useNotifications();
  const [demoStatus, setDemoStatus] = useState<'connected' | 'disconnected' | 'reconnecting'>('disconnected');

  const simulateConnection = () => {
    setDemoStatus('reconnecting');
    showToast({
      type: 'info',
      priority: 'medium',
      title: 'Connection Status Demo',
      message: 'Simulating connection attempt...',
      category: 'system',
      duration: 2000,
    });

    setTimeout(() => {
      setDemoStatus('connected');
      showToast({
        type: 'success',
        priority: 'medium',
        title: 'Connection Established',
        message: 'WebSocket connection simulated successfully!',
        category: 'system',
        duration: 3000,
      });
    }, 2000);
  };

  const simulateDisconnection = () => {
    setDemoStatus('disconnected');
    showToast({
      type: 'warning',
      priority: 'medium',
      title: 'Connection Lost',
      message: 'WebSocket connection has been disconnected.',
      category: 'system',
      duration: 3000,
    });
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-lg p-6">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
        Connection Status Demo
      </h3>
      
      <div className="space-y-4">
        {/* Status Display */}
        <div className="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-700 rounded-lg">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Current Status:
          </span>
          <ConnectionStatus size="md" />
        </div>

        {/* Demo Controls */}
        <div className="flex gap-3">
          <button
            onClick={simulateConnection}
            disabled={demoStatus === 'reconnecting'}
            className="flex-1 px-4 py-2 bg-green-500 text-white rounded-lg hover:bg-green-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
          >
            {demoStatus === 'reconnecting' ? 'Connecting...' : 'Simulate Connect'}
          </button>
          <button
            onClick={simulateDisconnection}
            disabled={demoStatus === 'disconnected'}
            className="flex-1 px-4 py-2 bg-red-500 text-white rounded-lg hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
          >
            Simulate Disconnect
          </button>
        </div>

        {/* Status Variants */}
        <div className="space-y-3">
          <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300">
            All Status Variants:
          </h4>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div className="flex items-center justify-center p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                <span className="text-sm text-green-700 dark:text-green-300">Connected</span>
              </div>
            </div>
            <div className="flex items-center justify-center p-3 bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-gray-400 rounded-full"></div>
                <span className="text-sm text-gray-600 dark:text-gray-400">Disconnected</span>
              </div>
            </div>
            <div className="flex items-center justify-center p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 bg-yellow-500 rounded-full animate-pulse"></div>
                <span className="text-sm text-yellow-700 dark:text-yellow-300">Reconnecting</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};