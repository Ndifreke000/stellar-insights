'use client';

import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { 
  Bell, 
  Settings, 
  Plus, 
  Send, 
  CheckCircle, 
  AlertCircle, 
  AlertTriangle, 
  Info,
  Trash2,
  RefreshCw
} from 'lucide-react';
import { useNotifications } from '@/contexts/NotificationContext';
import { NotificationService } from '@/services/notificationService';
import { BaseNotification, NotificationType, NotificationPriority } from '@/types/notifications';

export const NotificationCenterDemo: React.FC = () => {
  const { showToast, notifications, clearAllNotifications } = useNotifications();
  const notificationService = NotificationService.getInstance();
  const [isGenerating, setIsGenerating] = useState(false);

  const generateSampleNotifications = async () => {
    setIsGenerating(true);
    
    const samples: Array<{
      type: NotificationType;
      priority: NotificationPriority;
      title: string;
      message: string;
      category: keyof NotificationPreferences['categories'];
    }> = [
      {
        type: 'success',
        priority: 'medium',
        title: 'Payment Completed',
        message: 'Your payment of 100 XLM has been successfully processed.',
        category: 'payments'
      },
      {
        type: 'warning',
        priority: 'high',
        title: 'Low Liquidity Alert',
        message: 'Liquidity pool USDC/XLM has fallen below 15% capacity.',
        category: 'liquidity'
      },
      {
        type: 'error',
        priority: 'critical',
        title: 'Transaction Failed',
        message: 'Transaction could not be completed due to insufficient funds.',
        category: 'payments'
      },
      {
        type: 'info',
        priority: 'low',
        title: 'New Snapshot Available',
        message: 'Network snapshot for epoch #1234 is now available for analysis.',
        category: 'snapshots'
      },
      {
        type: 'warning',
        priority: 'medium',
        title: 'System Maintenance',
        message: 'Scheduled maintenance will begin in 2 hours. Expect brief downtime.',
        category: 'system'
      },
      {
        type: 'success',
        priority: 'low',
        title: 'Account Verified',
        message: 'Your Stellar account has been successfully verified.',
        category: 'system'
      },
      {
        type: 'info',
        priority: 'medium',
        title: 'Market Update',
        message: 'XLM price has increased by 5.2% in the last hour.',
        category: 'liquidity'
      },
      {
        type: 'error',
        priority: 'high',
        title: 'Connection Lost',
        message: 'Lost connection to Stellar network. Attempting to reconnect...',
        category: 'system'
      }
    ];

    // Generate notifications with delays for realistic effect
    for (let i = 0; i < samples.length; i++) {
      await new Promise(resolve => setTimeout(resolve, i * 300));
      const sample = samples[i];
      showToast(sample);
    }

    setIsGenerating(false);
  };

  const generateTemplateNotifications = async () => {
    setIsGenerating(true);
    
    // Use template system
    const paymentSuccess = notificationService.createFromTemplate('payment-success', {
      amount: '250',
      recipient: 'GD5DQ...K3F7',
      duration: '2.3'
    });

    const paymentFailed = notificationService.createFromTemplate('payment-failed', {
      amount: '100',
      recipient: 'GABC1...XYZ9',
      reason: 'Insufficient funds'
    });

    const lowLiquidity = notificationService.createFromTemplate('low-liquidity', {
      pool: 'USDC/XLM',
      threshold: '12'
    });

    if (paymentSuccess) {
      await new Promise(resolve => setTimeout(resolve, 500));
      showToast(paymentSuccess);
    }

    if (paymentFailed) {
      await new Promise(resolve => setTimeout(resolve, 500));
      showToast(paymentFailed);
    }

    if (lowLiquidity) {
      await new Promise(resolve => setTimeout(resolve, 500));
      showToast(lowLiquidity);
    }

    setIsGenerating(false);
  };

  const clearAll = () => {
    clearAllNotifications();
  };

  const stats = {
    total: notifications.length,
    unread: notifications.filter(n => !n.read).length,
    byType: notifications.reduce((acc, n) => {
      acc[n.type] = (acc[n.type] || 0) + 1;
      return acc;
    }, {} as Record<NotificationType, number>),
    byPriority: notifications.reduce((acc, n) => {
      acc[n.priority] = (acc[n.priority] || 0) + 1;
      return acc;
    }, {} as Record<NotificationPriority, number>)
  };

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="bg-white dark:bg-gray-900 rounded-xl shadow-lg border border-gray-200 dark:border-gray-700"
      >
        {/* Header */}
        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <Bell className="h-8 w-8 text-blue-500" />
              <div>
                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
                  Notification Center Demo
                </h1>
                <p className="text-muted-foreground dark:text-muted-foreground">
                  Test the real-time notification system
                </p>
              </div>
            </div>
            <div className="flex items-center space-x-2">
              <div className="text-right">
                <p className="text-sm text-muted-foreground dark:text-muted-foreground">Current Notifications</p>
                <p className="text-2xl font-bold text-gray-900 dark:text-white">{stats.total}</p>
              </div>
            </div>
          </div>
        </div>

        {/* Stats Grid */}
        <div className="p-6 grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-gray-50 dark:bg-gray-800 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground dark:text-muted-foreground">Total</span>
              <span className="text-lg font-bold text-gray-900 dark:text-white">{stats.total}</span>
            </div>
          </div>
          <div className="bg-red-50 dark:bg-red-900/20 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <span className="text-sm text-red-600 dark:text-red-400">Unread</span>
              <span className="text-lg font-bold text-red-600 dark:text-red-400">{stats.unread}</span>
            </div>
          </div>
          <div className="bg-green-50 dark:bg-green-900/20 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <span className="text-sm text-green-600 dark:text-green-400">Success</span>
              <span className="text-lg font-bold text-green-600 dark:text-green-400">{stats.byType.success || 0}</span>
            </div>
          </div>
          <div className="bg-orange-50 dark:bg-orange-900/20 p-4 rounded-lg">
            <div className="flex items-center justify-between">
              <span className="text-sm text-orange-600 dark:text-orange-400">Critical</span>
              <span className="text-lg font-bold text-orange-600 dark:text-orange-400">{stats.byPriority.critical || 0}</span>
            </div>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="p-6 border-t border-gray-200 dark:border-gray-700">
          <div className="flex flex-wrap gap-4">
            <button
              onClick={generateSampleNotifications}
              disabled={isGenerating}
              className="flex items-center space-x-2 px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {isGenerating ? (
                <RefreshCw className="h-4 w-4 animate-spin" />
              ) : (
                <Plus className="h-4 w-4" />
              )}
              <span>Generate Sample Notifications</span>
            </button>

            <button
              onClick={generateTemplateNotifications}
              disabled={isGenerating}
              className="flex items-center space-x-2 px-4 py-2 bg-purple-500 text-white rounded-lg hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {isGenerating ? (
                <RefreshCw className="h-4 w-4 animate-spin" />
              ) : (
                <Send className="h-4 w-4" />
              )}
              <span>Generate Template Notifications</span>
            </button>

            <button
              onClick={clearAll}
              className="flex items-center space-x-2 px-4 py-2 bg-red-500 text-white rounded-lg hover:bg-red-600 transition-colors"
            >
              <Trash2 className="h-4 w-4" />
              <span>Clear All</span>
            </button>
          </div>
        </div>

        {/* Recent Notifications Preview */}
        {notifications.length > 0 && (
          <div className="p-6 border-t border-gray-200 dark:border-gray-700">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              Recent Notifications
            </h3>
            <div className="space-y-2 max-h-64 overflow-y-auto">
              {notifications.slice(0, 5).map((notification) => {
                const icons = {
                  success: CheckCircle,
                  error: AlertCircle,
                  warning: AlertTriangle,
                  info: Info,
                };
                const colors = {
                  success: 'text-green-500',
                  error: 'text-red-500',
                  warning: 'text-yellow-500',
                  info: 'text-blue-500',
                };
                const Icon = icons[notification.type];
                
                return (
                  <div
                    key={notification.id}
                    className={`p-3 rounded-lg border ${
                      notification.read
                        ? 'bg-gray-50 dark:bg-gray-800 border-gray-200 dark:border-gray-700'
                        : 'bg-white dark:bg-gray-900 border-blue-200 dark:border-blue-800'
                    }`}
                  >
                    <div className="flex items-start space-x-3">
                      <Icon className={`h-5 w-5 mt-0.5 ${colors[notification.type]}`} />
                      <div className="flex-1 min-w-0">
                        <h4 className="text-sm font-medium text-gray-900 dark:text-white">
                          {notification.title}
                        </h4>
                        <p className="text-sm text-muted-foreground dark:text-muted-foreground mt-1">
                          {notification.message}
                        </p>
                        <div className="flex items-center space-x-2 mt-2">
                          <span className="text-xs text-muted-foreground">
                            {new Date(notification.timestamp).toLocaleTimeString()}
                          </span>
                          <span className={`text-xs px-2 py-1 rounded-full ${
                            notification.priority === 'critical' ? 'bg-red-100 text-red-800' :
                            notification.priority === 'high' ? 'bg-orange-100 text-orange-800' :
                            notification.priority === 'medium' ? 'bg-blue-100 text-blue-800' :
                            'bg-gray-100 text-gray-800'
                          }`}>
                            {notification.priority}
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Instructions */}
        <div className="p-6 border-t border-gray-200 dark:border-gray-700">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">
            How to Use
          </h3>
          <div className="space-y-2 text-sm text-muted-foreground dark:text-muted-foreground">
            <p>• Click the notification bell in the navbar to open the notification center</p>
            <p>• Use the demo buttons above to generate sample notifications</p>
            <p>• Test filtering, searching, and batch operations in the notification center</p>
            <p>• Switch to the Analytics tab to view notification metrics</p>
            <p>• Use the Settings button to configure notification preferences</p>
          </div>
        </div>
      </motion.div>
    </div>
  );
};
