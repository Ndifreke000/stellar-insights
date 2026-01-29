'use client';

import React, { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Bell, 
  BellOff, 
  CheckCircle, 
  AlertCircle, 
  AlertTriangle, 
  Info,
  Trash2,
  Filter,
  Search,
  Calendar,
  X
} from 'lucide-react';
import { format, isToday, isYesterday, subDays } from 'date-fns';
import { BaseNotification, NotificationType, NotificationPriority } from '@/types/notifications';
import { useNotifications } from '@/contexts/NotificationContext';

interface NotificationHistoryProps {
  isOpen: boolean;
  onClose: () => void;
}

const NOTIFICATION_ICONS: Record<NotificationType, React.ComponentType<{ className?: string }>> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const ICON_COLORS: Record<NotificationType, string> = {
  success: 'text-green-500',
  error: 'text-red-500',
  warning: 'text-yellow-500',
  info: 'text-blue-500',
};

const PRIORITY_BADGES: Record<NotificationPriority, { label: string; className: string }> = {
  low: { label: 'Low', className: 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200' },
  medium: { label: 'Medium', className: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200' },
  high: { label: 'High', className: 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200' },
  critical: { label: 'Critical', className: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200' },
};

type FilterType = 'all' | NotificationType;
type SortType = 'newest' | 'oldest' | 'priority';

export const NotificationHistory: React.FC<NotificationHistoryProps> = ({ isOpen, onClose }) => {
  const { 
    notifications, 
    markAsRead, 
    markAllAsRead, 
    clearNotification, 
    clearAllNotifications,
    unreadCount 
  } = useNotifications();

  const [searchTerm, setSearchTerm] = useState('');
  const [filterType, setFilterType] = useState<FilterType>('all');
  const [sortType, setSortType] = useState<SortType>('newest');
  const [showUnreadOnly, setShowUnreadOnly] = useState(false);

  const filteredAndSortedNotifications = useMemo(() => {
    let filtered = notifications;

    // Apply search filter
    if (searchTerm) {
      filtered = filtered.filter(notification =>
        notification.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
        notification.message.toLowerCase().includes(searchTerm.toLowerCase())
      );
    }

    // Apply type filter
    if (filterType !== 'all') {
      filtered = filtered.filter(notification => notification.type === filterType);
    }

    // Apply unread filter
    if (showUnreadOnly) {
      filtered = filtered.filter(notification => !notification.read);
    }

    // Apply sorting
    filtered.sort((a, b) => {
      switch (sortType) {
        case 'oldest':
          return new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime();
        case 'priority':
          const priorityOrder = { critical: 4, high: 3, medium: 2, low: 1 };
          const priorityDiff = priorityOrder[b.priority] - priorityOrder[a.priority];
          if (priorityDiff !== 0) return priorityDiff;
          return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
        case 'newest':
        default:
          return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
      }
    });

    return filtered;
  }, [notifications, searchTerm, filterType, sortType, showUnreadOnly]);

  const groupedNotifications = useMemo(() => {
    const groups: Record<string, BaseNotification[]> = {};
    
    filteredAndSortedNotifications.forEach(notification => {
      const date = new Date(notification.timestamp);
      let groupKey: string;

      if (isToday(date)) {
        groupKey = 'Today';
      } else if (isYesterday(date)) {
        groupKey = 'Yesterday';
      } else if (date >= subDays(new Date(), 7)) {
        groupKey = 'This Week';
      } else {
        groupKey = format(date, 'MMMM yyyy');
      }

      if (!groups[groupKey]) {
        groups[groupKey] = [];
      }
      groups[groupKey].push(notification);
    });

    return groups;
  }, [filteredAndSortedNotifications]);

  const formatTime = (date: Date) => {
    if (isToday(date)) {
      return format(date, 'HH:mm');
    } else if (isYesterday(date)) {
      return `Yesterday ${format(date, 'HH:mm')}`;
    } else {
      return format(date, 'MMM dd, HH:mm');
    }
  };

  const handleNotificationClick = (notification: BaseNotification) => {
    if (!notification.read) {
      markAsRead(notification.id);
    }
  };

  if (!isOpen) return null;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      >
        <motion.div
          initial={{ x: '100%' }}
          animate={{ x: 0 }}
          exit={{ x: '100%' }}
          transition={{ type: 'spring', damping: 30, stiffness: 300 }}
          className="absolute right-0 top-0 h-full w-full max-w-md bg-white dark:bg-gray-900 shadow-xl"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="border-b border-gray-200 dark:border-gray-700 p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <Bell className="h-5 w-5 text-gray-600 dark:text-gray-400" />
                <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                  Notifications
                </h2>
                {unreadCount > 0 && (
                  <span className="bg-red-500 text-white text-xs px-2 py-1 rounded-full">
                    {unreadCount}
                  </span>
                )}
              </div>
              <button
                onClick={onClose}
                className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            {/* Search and Filters */}
            <div className="mt-4 space-y-3">
              {/* Search */}
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                <input
                  type="text"
                  placeholder="Search notifications..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  className="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-500 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
              </div>

              {/* Filter Controls */}
              <div className="flex flex-wrap gap-2">
                <select
                  value={filterType}
                  onChange={(e) => setFilterType(e.target.value as FilterType)}
                  className="px-3 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                >
                  <option value="all">All Types</option>
                  <option value="success">Success</option>
                  <option value="error">Error</option>
                  <option value="warning">Warning</option>
                  <option value="info">Info</option>
                </select>

                <select
                  value={sortType}
                  onChange={(e) => setSortType(e.target.value as SortType)}
                  className="px-3 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                >
                  <option value="newest">Newest First</option>
                  <option value="oldest">Oldest First</option>
                  <option value="priority">By Priority</option>
                </select>

                <button
                  onClick={() => setShowUnreadOnly(!showUnreadOnly)}
                  className={`px-3 py-1 text-sm rounded-md transition-colors ${
                    showUnreadOnly
                      ? 'bg-blue-500 text-white'
                      : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300'
                  }`}
                >
                  Unread Only
                </button>
              </div>

              {/* Action Buttons */}
              <div className="flex space-x-2">
                <button
                  onClick={markAllAsRead}
                  disabled={unreadCount === 0}
                  className="flex-1 px-3 py-2 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                >
                  Mark All Read
                </button>
                <button
                  onClick={clearAllNotifications}
                  className="flex-1 px-3 py-2 text-sm bg-red-500 text-white rounded-lg hover:bg-red-600 transition-colors"
                >
                  Clear All
                </button>
              </div>
            </div>
          </div>

          {/* Notifications List */}
          <div className="flex-1 overflow-y-auto">
            {Object.keys(groupedNotifications).length === 0 ? (
              <div className="flex flex-col items-center justify-center h-64 text-gray-500 dark:text-gray-400">
                <BellOff className="h-12 w-12 mb-4" />
                <p className="text-lg font-medium">No notifications</p>
                <p className="text-sm">You're all caught up!</p>
              </div>
            ) : (
              <div className="p-4 space-y-6">
                {Object.entries(groupedNotifications).map(([groupKey, groupNotifications]) => (
                  <div key={groupKey}>
                    <h3 className="text-sm font-medium text-gray-500 dark:text-gray-400 mb-3 flex items-center">
                      <Calendar className="h-4 w-4 mr-2" />
                      {groupKey}
                    </h3>
                    <div className="space-y-2">
                      {groupNotifications.map((notification) => {
                        const Icon = NOTIFICATION_ICONS[notification.type];
                        return (
                          <motion.div
                            key={notification.id}
                            layout
                            initial={{ opacity: 0, y: 20 }}
                            animate={{ opacity: 1, y: 0 }}
                            exit={{ opacity: 0, y: -20 }}
                            className={`
                              p-3 rounded-lg border cursor-pointer transition-all hover:shadow-md
                              ${notification.read 
                                ? 'bg-gray-50 dark:bg-gray-800 border-gray-200 dark:border-gray-700' 
                                : 'bg-white dark:bg-gray-900 border-blue-200 dark:border-blue-800 shadow-sm'
                              }
                            `}
                            onClick={() => handleNotificationClick(notification)}
                          >
                            <div className="flex items-start space-x-3">
                              <Icon className={`h-5 w-5 mt-0.5 ${ICON_COLORS[notification.type]}`} />
                              <div className="flex-1 min-w-0">
                                <div className="flex items-center justify-between">
                                  <h4 className={`text-sm font-medium ${
                                    notification.read 
                                      ? 'text-gray-700 dark:text-gray-300' 
                                      : 'text-gray-900 dark:text-white'
                                  }`}>
                                    {notification.title}
                                  </h4>
                                  <div className="flex items-center space-x-2">
                                    <span className={`px-2 py-1 text-xs rounded-full ${PRIORITY_BADGES[notification.priority].className}`}>
                                      {PRIORITY_BADGES[notification.priority].label}
                                    </span>
                                    <button
                                      onClick={(e) => {
                                        e.stopPropagation();
                                        clearNotification(notification.id);
                                      }}
                                      className="p-1 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors"
                                    >
                                      <Trash2 className="h-3 w-3 text-gray-400" />
                                    </button>
                                  </div>
                                </div>
                                <p className={`text-sm mt-1 ${
                                  notification.read 
                                    ? 'text-gray-500 dark:text-gray-400' 
                                    : 'text-gray-700 dark:text-gray-300'
                                }`}>
                                  {notification.message}
                                </p>
                                <p className="text-xs text-gray-400 mt-2">
                                  {formatTime(new Date(notification.timestamp))}
                                </p>
                              </div>
                            </div>
                          </motion.div>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
};