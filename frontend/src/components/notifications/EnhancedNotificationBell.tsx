'use client';

import React, { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Bell, BellOff, X, CheckCircle, AlertCircle, AlertTriangle, Info } from 'lucide-react';
import { BaseNotification, NotificationType, NotificationPriority } from '@/types/notifications';
import { useNotifications } from '@/contexts/NotificationContext';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

interface NotificationBellProps {
  className?: string;
}

const NOTIFICATION_ICONS: Record<NotificationType, React.ComponentType<{ className?: string }>> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const TYPE_COLORS: Record<NotificationType, string> = {
  success: 'text-green-500',
  error: 'text-red-500',
  warning: 'text-yellow-500',
  info: 'text-blue-500',
};

const PRIORITY_COLORS: Record<NotificationPriority, string> = {
  low: 'bg-gray-500',
  medium: 'bg-blue-500',
  high: 'bg-orange-500',
  critical: 'bg-red-500 animate-pulse',
};

export const EnhancedNotificationBell: React.FC<NotificationBellProps> = ({ className = '' }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const { unreadCount, notifications, markAsRead, markAllAsRead, isWebSocketConnected } = useNotifications();
  const bellRef = useRef<HTMLButtonElement>(null);
  const previewTimeoutRef = useRef<NodeJS.Timeout>();

  // Get recent unread notifications for preview
  const recentUnread = notifications
    .filter(n => !n.read)
    .slice(0, 5);

  // Handle bell click
  const handleBellClick = () => {
    setIsOpen(!isOpen);
    if (!isOpen && unreadCount > 0) {
      markAllAsRead();
    }
  };

  // Show preview on hover
  const handleMouseEnter = () => {
    if (unreadCount > 0) {
      previewTimeoutRef.current = setTimeout(() => {
        setShowPreview(true);
      }, 500);
    }
  };

  const handleMouseLeave = () => {
    if (previewTimeoutRef.current) {
      clearTimeout(previewTimeoutRef.current);
    }
    setShowPreview(false);
  };

  // Close preview when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (bellRef.current && !bellRef.current.contains(event.target as Node)) {
        setShowPreview(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  return (
    <TooltipProvider>
      <div className="relative" ref={bellRef}>
        {/* Notification Bell */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleBellClick}
              onMouseEnter={handleMouseEnter}
              onMouseLeave={handleMouseLeave}
              className={`relative p-2 ${className}`}
            >
              <div className="relative">
                <Bell className={`h-5 w-5 transition-colors ${unreadCount > 0 ? 'text-blue-600 dark:text-blue-400' : 'text-gray-500'
                  }`} />

                {/* Unread Badge */}
                {unreadCount > 0 && (
                  <Badge
                    className={`absolute -top-1 -right-1 min-w-[18px] h-[18px] text-xs flex items-center justify-center p-0 border-2 border-white dark:border-slate-900 ${unreadCount > 0 ? PRIORITY_COLORS[getHighestPriority(recentUnread)] : ''
                      }`}
                  >
                    {unreadCount > 99 ? '99+' : unreadCount}
                  </Badge>
                )}

                {/* Connection Status Indicator */}
                <div className={`absolute -bottom-1 -right-1 w-2 h-2 rounded-full border border-white dark:border-slate-900 ${isWebSocketConnected ? 'bg-green-500' : 'bg-red-500'
                  }`} />
              </div>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <div className="text-center">
              <div className="font-medium">Notifications</div>
              <div className="text-xs text-gray-500">
                {unreadCount > 0 ? `${unreadCount} unread` : 'No new notifications'}
              </div>
              <div className="text-xs text-gray-500">
                {isWebSocketConnected ? 'Connected' : 'Disconnected'}
              </div>
            </div>
          </TooltipContent>
        </Tooltip>

        {/* Notification Preview */}
        <AnimatePresence>
          {showPreview && recentUnread.length > 0 && (
            <motion.div
              initial={{ opacity: 0, y: -10, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -10, scale: 0.95 }}
              transition={{ duration: 0.2 }}
              className="absolute top-full right-0 mt-2 w-80 bg-white dark:bg-slate-900 rounded-lg shadow-lg border border-gray-200 dark:border-slate-700 z-50"
            >
              {/* Preview Header */}
              <div className="flex items-center justify-between p-3 border-b border-gray-200 dark:border-slate-700">
                <div className="font-medium text-sm">Recent Notifications</div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    markAllAsRead();
                    setShowPreview(false);
                  }}
                  className="text-xs"
                >
                  Mark all read
                </Button>
              </div>

              {/* Notification List */}
              <div className="max-h-64 overflow-y-auto">
                {recentUnread.map((notification, index) => (
                  <motion.div
                    key={notification.id}
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ delay: index * 0.05 }}
                    className="p-3 border-b border-gray-100 dark:border-slate-800 last:border-b-0 hover:bg-gray-50 dark:hover:bg-slate-800 cursor-pointer transition-colors"
                    onClick={() => {
                      markAsRead(notification.id);
                      setShowPreview(false);
                      setIsOpen(true);
                    }}
                  >
                    <div className="flex items-start gap-3">
                      {React.createElement(NOTIFICATION_ICONS[notification.type], {
                        className: `h-4 w-4 mt-0.5 shrink-0 ${TYPE_COLORS[notification.type]}`
                      })}
                      <div className="shrink-0 mt-0.5">
                        <div className="flex items-center gap-2 mb-1">
                          <h4 className="font-medium text-sm truncate">{notification.title}</h4>
                          <Badge variant="secondary" className="text-xs">
                            {notification.priority}
                          </Badge>
                        </div>
                        <p className="text-xs text-gray-600 dark:text-gray-400 line-clamp-2">
                          {notification.message}
                        </p>
                        <div className="text-xs text-gray-500 mt-1">
                          {formatRelativeTime(notification.timestamp)}
                        </div>
                      </div>
                    </div>
                  </motion.div>
                ))}
              </div>

              {/* Preview Footer */}
              <div className="p-3 border-t border-gray-200 dark:border-slate-700">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setShowPreview(false);
                    setIsOpen(true);
                  }}
                  className="w-full"
                >
                  View all {unreadCount} notifications
                </Button>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Quick Actions Panel */}
        <AnimatePresence>
          {isOpen && (
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              className="absolute top-full right-0 mt-2 w-96 bg-white dark:bg-slate-900 rounded-lg shadow-lg border border-gray-200 dark:border-slate-700 z-50"
            >
              <div className="p-4">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="font-semibold">Quick Actions</h3>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setIsOpen(false)}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>

                <div className="space-y-3">
                  <Button
                    variant="outline"
                    className="w-full justify-start"
                    onClick={() => {
                      markAllAsRead();
                      setIsOpen(false);
                    }}
                    disabled={unreadCount === 0}
                  >
                    <CheckCircle className="h-4 w-4 mr-2" />
                    Mark all as read
                  </Button>

                  <Button
                    variant="outline"
                    className="w-full justify-start"
                    onClick={() => {
                      // Open full notification center
                      setIsOpen(false);
                      // This would typically open the main notification center
                      window.dispatchEvent(new CustomEvent('openNotificationCenter'));
                    }}
                  >
                    <Bell className="h-4 w-4 mr-2" />
                    Open notification center
                  </Button>

                  <Button
                    variant="outline"
                    className="w-full justify-start"
                    onClick={() => {
                      // Open notification settings
                      setIsOpen(false);
                      // This would typically open settings
                      window.dispatchEvent(new CustomEvent('openNotificationSettings'));
                    }}
                  >
                    Settings
                  </Button>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </TooltipProvider>
  );
};

// Helper function to get highest priority from notifications
function getHighestPriority(notifications: BaseNotification[]): NotificationPriority {
  if (notifications.length === 0) return 'low';

  const priorityOrder = { critical: 4, high: 3, medium: 2, low: 1 };

  return notifications.reduce((highest, notification) => {
    return priorityOrder[notification.priority] > priorityOrder[highest]
      ? notification.priority
      : highest;
  }, 'low' as NotificationPriority);
}

// Helper function to format relative time
function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffInSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (diffInSeconds < 60) {
    return 'just now';
  } else if (diffInSeconds < 3600) {
    const minutes = Math.floor(diffInSeconds / 60);
    return `${minutes}m ago`;
  } else if (diffInSeconds < 86400) {
    const hours = Math.floor(diffInSeconds / 3600);
    return `${hours}h ago`;
  } else {
    const days = Math.floor(diffInSeconds / 86400);
    return `${days}d ago`;
  }
}
