'use client';

import React from 'react';
import { Wallet, WalletCards, CheckCircle, XCircle } from 'lucide-react';
import { useWallet } from '../lib/wallet-context';
import { useNotifications } from '@/contexts/NotificationContext';

export const WalletConnectionDemo: React.FC = () => {
  const { isConnected, address, isConnecting, connectWallet, disconnectWallet } = useWallet();
  const { showToast } = useNotifications();

  const displayAddress = address
    ? `${address.slice(0, 6)}...${address.slice(-4)}`
    : null;

  const handleTestConnect = async () => {
    try {
      await connectWallet();
    } catch (error) {
      // Error notification is already handled in the header
    }
  };

  const handleTestDisconnect = () => {
    disconnectWallet();
  };

  const simulateConnectionError = () => {
    showToast({
      type: 'error',
      priority: 'high',
      title: 'Connection Failed',
      message: 'Failed to connect to wallet. Please check your wallet extension.',
      category: 'system',
      duration: 5000,
      actions: [
        {
          id: 'retry',
          label: 'Retry',
          variant: 'primary',
          onClick: handleTestConnect,
        },
      ],
    });
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-lg p-6">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
        <WalletCards className="h-5 w-5" />
        Wallet Connection Demo
      </h3>
      
      <div className="space-y-4">
        {/* Current Status */}
        <div className="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-700 rounded-lg">
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-full ${
              isConnected 
                ? 'bg-green-100 dark:bg-green-900/20' 
                : 'bg-gray-100 dark:bg-gray-800'
            }`}>
              {isConnected ? (
                <CheckCircle className="h-5 w-5 text-green-600 dark:text-green-400" />
              ) : (
                <XCircle className="h-5 w-5 text-gray-400" />
              )}
            </div>
            <div>
              <p className="font-medium text-gray-900 dark:text-white">
                {isConnected ? 'Connected' : 'Disconnected'}
              </p>
              {isConnected && displayAddress && (
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  {displayAddress}
                </p>
              )}
            </div>
          </div>
          <div className={`px-3 py-1 rounded-full text-xs font-medium ${
            isConnected
              ? 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-200'
              : 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200'
          }`}>
            {isConnected ? 'Active' : 'Inactive'}
          </div>
        </div>

        {/* Action Buttons */}
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <button
            onClick={handleTestConnect}
            disabled={isConnected || isConnecting}
            className="flex items-center justify-center gap-2 px-4 py-3 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
          >
            <Wallet className="h-4 w-4" />
            {isConnecting ? 'Connecting...' : 'Connect'}
          </button>
          
          <button
            onClick={handleTestDisconnect}
            disabled={!isConnected}
            className="flex items-center justify-center gap-2 px-4 py-3 bg-red-500 text-white rounded-lg hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm font-medium"
          >
            <XCircle className="h-4 w-4" />
            Disconnect
          </button>
          
          <button
            onClick={simulateConnectionError}
            className="flex items-center justify-center gap-2 px-4 py-3 bg-orange-500 text-white rounded-lg hover:bg-orange-600 transition-colors text-sm font-medium"
          >
            <XCircle className="h-4 w-4" />
            Simulate Error
          </button>
        </div>

        {/* Instructions */}
        <div className="p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
          <h4 className="font-medium text-blue-900 dark:text-blue-200 mb-2">
            How to Test
          </h4>
          <ul className="text-sm text-blue-700 dark:text-blue-300 space-y-1">
            <li>• <span className="font-medium">Connect</span> - Shows success toast with wallet address</li>
            <li>• <span className="font-medium">Disconnect</span> - Shows info toast about disconnection</li>
            <li>• <span className="font-medium">Simulate Error</span> - Shows error toast with retry action</li>
            <li>• <span className="font-medium">Header Button</span> - Also triggers the same notifications</li>
          </ul>
        </div>

        {/* Notification Types */}
        <div className="space-y-3">
          <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Notification Types:
          </h4>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div className="flex items-center justify-center p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg">
              <div className="flex items-center gap-2">
                <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                <span className="text-sm text-green-700 dark:text-green-300 font-medium">Connected</span>
              </div>
            </div>
            <div className="flex items-center justify-center p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
              <div className="flex items-center gap-2">
                <XCircle className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                <span className="text-sm text-blue-700 dark:text-blue-300 font-medium">Disconnected</span>
              </div>
            </div>
            <div className="flex items-center justify-center p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
              <div className="flex items-center gap-2">
                <XCircle className="h-4 w-4 text-red-600 dark:text-red-400" />
                <span className="text-sm text-red-700 dark:text-red-300 font-medium">Error</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};