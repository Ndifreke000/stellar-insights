'use client';

import React from 'react';
import { motion } from 'framer-motion';
import { 
  Bell, 
  AlertTriangle, 
  CheckCircle, 
  Info, 
  AlertCircle,
  Zap,
  TrendingDown,
  Camera
} from 'lucide-react';
import { useNotifications } from '@/contexts/NotificationContext';
import { WalletConnectionDemo } from '@/components/notifications/WalletConnectionDemo';

export default function NotificationsDemo() {
  const { showToast } = useNotifications();

  const demoNotifications = [
    {
      title: 'Payment Failed',
      message: 'Payment of 1,000 USDC from Anchor A to Anchor B failed due to insufficient liquidity',
      type: 'error' as const,
      priority: 'high' as const,
      category: 'payments' as const,
      icon: AlertCircle,
      actions: [
        {
          id: 'retry',
          label: 'Retry Payment',
          variant: 'primary' as const,
          onClick: () => alert('Retrying payment...'),
        },
        {
          id: 'details',
          label: 'View Details',
          variant: 'secondary' as const,
          onClick: () => alert('Showing payment details...'),
        },
      ],
    },
    {
      title: 'Low Liquidity Alert',
      message: 'USD/EUR corridor liquidity has dropped below 50,000 USDC threshold',
      type: 'warning' as const,
      priority: 'medium' as const,
      category: 'liquidity' as const,
      icon: TrendingDown,
      actions: [
        {
          id: 'add-liquidity',
          label: 'Add Liquidity',
          variant: 'primary' as const,
          onClick: () => alert('Adding liquidity...'),
        },
      ],
    },
    {
      title: 'New Snapshot Available',
      message: 'Network snapshot #12,345 has been generated and is ready for analysis',
      type: 'info' as const,
      priority: 'low' as const,
      category: 'snapshots' as const,
      icon: Camera,
      actions: [
        {
          id: 'view-snapshot',
          label: 'View Snapshot',
          variant: 'primary' as const,
          onClick: () => alert('Opening snapshot...'),
        },
      ],
    },
    {
      title: 'System Maintenance Complete',
      message: 'Scheduled maintenance has been completed successfully. All systems are operational.',
      type: 'success' as const,
      priority: 'medium' as const,
      category: 'system' as const,
      icon: CheckCircle,
    },
    {
      title: 'Critical System Alert',
      message: 'Database connection lost. Attempting to reconnect...',
      type: 'error' as const,
      priority: 'critical' as const,
      category: 'system' as const,
      icon: Zap,
      persistent: true,
      actions: [
        {
          id: 'check-status',
          label: 'Check Status',
          variant: 'primary' as const,
          onClick: () => alert('Checking system status...'),
        },
        {
          id: 'contact-support',
          label: 'Contact Support',
          variant: 'destructive' as const,
          onClick: () => alert('Contacting support...'),
        },
      ],
    },
  ];

  const handleShowNotification = (notification: typeof demoNotifications[0]) => {
    showToast({
      title: notification.title,
      message: notification.message,
      type: notification.type,
      priority: notification.priority,
      category: notification.category,
      actions: notification.actions,
      persistent: notification.persistent,
    });
  };

  const handleShowRandomNotification = () => {
    const randomNotification = demoNotifications[Math.floor(Math.random() * demoNotifications.length)];
    handleShowNotification(randomNotification);
  };

  const handleShowMultipleNotifications = () => {
    demoNotifications.forEach((notification, index) => {
      setTimeout(() => {
        handleShowNotification(notification);
      }, index * 500);
    });
  };

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 py-8">
      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
        {/* Header */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-center mb-12"
        >
          <div className="flex items-center justify-center mb-4">
            <Bell className="h-12 w-12 text-blue-500" />
          </div>
          <h1 className="text-4xl font-bold text-gray-900 dark:text-white mb-4">
            Notification System Demo
          </h1>
          <p className="text-xl text-gray-600 dark:text-gray-400 max-w-2xl mx-auto">
            Experience our comprehensive toast notification system with real-time WebSocket support, 
            sound alerts, and customizable preferences.
          </p>
        </motion.div>

        {/* Quick Actions */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-white dark:bg-gray-800 rounded-xl shadow-lg p-6 mb-8"
        >
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-6">
            Quick Actions
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            <button
              onClick={handleShowRandomNotification}
              className="p-4 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors font-medium"
            >
              Show Random Notification
            </button>
            <button
              onClick={handleShowMultipleNotifications}
              className="p-4 bg-purple-500 text-white rounded-lg hover:bg-purple-600 transition-colors font-medium"
            >
              Show All Notifications
            </button>
            <button
              onClick={() => handleShowNotification(demoNotifications[4])}
              className="p-4 bg-red-500 text-white rounded-lg hover:bg-red-600 transition-colors font-medium"
            >
              Show Critical Alert
            </button>
          </div>
          
          {/* Wallet Connection Test */}
          <div className="mt-6 p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg">
            <h3 className="font-medium text-green-900 dark:text-green-200 mb-2">
              Test Wallet Connection Notifications
            </h3>
            <p className="text-sm text-green-700 dark:text-green-300 mb-3">
              Use the "Connect Wallet" button in the header to test connection/disconnection notifications.
            </p>
            <ul className="text-sm text-green-700 dark:text-green-300 space-y-1">
              <li>• <span className="font-medium">Connect</span> - Shows success notification with wallet address</li>
              <li>• <span className="font-medium">Disconnect</span> - Shows info notification about disconnection</li>
              <li>• <span className="font-medium">Error</span> - Shows error notification if connection fails</li>
            </ul>
          </div>
          
          {/* Connection Status Info */}
          <div className="mt-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
            <h3 className="font-medium text-blue-900 dark:text-blue-200 mb-2">
              WebSocket Connection Status
            </h3>
            <p className="text-sm text-blue-700 dark:text-blue-300 mb-3">
              Check the header to see the real-time connection status. The status shows:
            </p>
            <ul className="text-sm text-blue-700 dark:text-blue-300 space-y-1">
              <li>• <span className="font-medium">Connected</span> - WebSocket is active (green)</li>
              <li>• <span className="font-medium">Disconnected</span> - No WebSocket connection (gray)</li>
              <li>• <span className="font-medium">Reconnecting</span> - Attempting to reconnect (yellow, spinning)</li>
            </ul>
            <p className="text-xs text-blue-600 dark:text-blue-400 mt-3">
              To enable WebSocket: Set NEXT_PUBLIC_WS_URL in your .env.local file
            </p>
          </div>
        </motion.div>

        {/* Wallet Connection Demo */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.25 }}
          className="mb-8"
        >
          <WalletConnectionDemo />
        </motion.div>

        {/* Notification Types */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-white dark:bg-gray-800 rounded-xl shadow-lg p-6"
        >
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-6">
            Notification Types
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {demoNotifications.map((notification, index) => {
              const Icon = notification.icon;
              return (
                <motion.div
                  key={index}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: 0.3 + index * 0.1 }}
                  className="border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:shadow-md transition-shadow"
                >
                  <div className="flex items-start space-x-3 mb-3">
                    <Icon className={`h-6 w-6 mt-1 ${
                      notification.type === 'success' ? 'text-green-500' :
                      notification.type === 'error' ? 'text-red-500' :
                      notification.type === 'warning' ? 'text-yellow-500' :
                      'text-blue-500'
                    }`} />
                    <div className="flex-1">
                      <h3 className="font-medium text-gray-900 dark:text-white">
                        {notification.title}
                      </h3>
                      <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                        {notification.message}
                      </p>
                      <div className="flex items-center space-x-2 mt-2">
                        <span className={`px-2 py-1 text-xs rounded-full ${
                          notification.type === 'success' ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' :
                          notification.type === 'error' ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200' :
                          notification.type === 'warning' ? 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200' :
                          'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
                        }`}>
                          {notification.type}
                        </span>
                        <span className={`px-2 py-1 text-xs rounded-full ${
                          notification.priority === 'critical' ? 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200' :
                          notification.priority === 'high' ? 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200' :
                          notification.priority === 'medium' ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200' :
                          'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200'
                        }`}>
                          {notification.priority} priority
                        </span>
                      </div>
                    </div>
                  </div>
                  <button
                    onClick={() => handleShowNotification(notification)}
                    className="w-full px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors text-sm font-medium"
                  >
                    Show This Notification
                  </button>
                </motion.div>
              );
            })}
          </div>
        </motion.div>

        {/* Features */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="mt-8 bg-white dark:bg-gray-800 rounded-xl shadow-lg p-6"
        >
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-6">
            Features
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <div className="text-center">
              <div className="bg-blue-100 dark:bg-blue-900 rounded-full p-3 w-12 h-12 mx-auto mb-3 flex items-center justify-center">
                <Bell className="h-6 w-6 text-blue-600 dark:text-blue-400" />
              </div>
              <h3 className="font-medium text-gray-900 dark:text-white mb-2">Toast Variants</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Success, error, warning, and info toast notifications with custom styling
              </p>
            </div>
            <div className="text-center">
              <div className="bg-green-100 dark:bg-green-900 rounded-full p-3 w-12 h-12 mx-auto mb-3 flex items-center justify-center">
                <Zap className="h-6 w-6 text-green-600 dark:text-green-400" />
              </div>
              <h3 className="font-medium text-gray-900 dark:text-white mb-2">Real-time WebSocket</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Live notifications from server events with automatic reconnection
              </p>
            </div>
            <div className="text-center">
              <div className="bg-purple-100 dark:bg-purple-900 rounded-full p-3 w-12 h-12 mx-auto mb-3 flex items-center justify-center">
                <CheckCircle className="h-6 w-6 text-purple-600 dark:text-purple-400" />
              </div>
              <h3 className="font-medium text-gray-900 dark:text-white mb-2">Sound Alerts</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Customizable sound notifications with volume and type controls
              </p>
            </div>
            <div className="text-center">
              <div className="bg-yellow-100 dark:bg-yellow-900 rounded-full p-3 w-12 h-12 mx-auto mb-3 flex items-center justify-center">
                <Info className="h-6 w-6 text-yellow-600 dark:text-yellow-400" />
              </div>
              <h3 className="font-medium text-gray-900 dark:text-white mb-2">Notification History</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Complete history with search, filtering, and mark as read functionality
              </p>
            </div>
            <div className="text-center">
              <div className="bg-red-100 dark:bg-red-900 rounded-full p-3 w-12 h-12 mx-auto mb-3 flex items-center justify-center">
                <AlertTriangle className="h-6 w-6 text-red-600 dark:text-red-400" />
              </div>
              <h3 className="font-medium text-gray-900 dark:text-white mb-2">Priority System</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Low, medium, high, and critical priority levels with visual indicators
              </p>
            </div>
            <div className="text-center">
              <div className="bg-indigo-100 dark:bg-indigo-900 rounded-full p-3 w-12 h-12 mx-auto mb-3 flex items-center justify-center">
                <Camera className="h-6 w-6 text-indigo-600 dark:text-indigo-400" />
              </div>
              <h3 className="font-medium text-gray-900 dark:text-white mb-2">Preferences</h3>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                Comprehensive settings for customizing notification behavior
              </p>
            </div>
          </div>
        </motion.div>
      </div>
    </div>
  );
}