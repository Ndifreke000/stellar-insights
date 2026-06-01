'use client';

import React, { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Bell,
  BellOff,
  Settings,
  Download,
  Trash2,
  CheckCircle,
  AlertCircle,
  AlertTriangle,
  Info,
  Filter,
  Search,
  Calendar,
  X,
  BarChart3,
  Clock,
  TrendingUp,
  Users,
  Activity,
  ChevronDown,
  ChevronUp,
  RefreshCw
} from 'lucide-react';
import { format, isToday, isYesterday, subDays, startOfDay, endOfDay } from 'date-fns';
import { BaseNotification, NotificationType, NotificationPriority } from '@/types/notifications';
import { useNotifications } from '@/contexts/NotificationContext';
import { NotificationService, NotificationFilter, NotificationAnalytics } from '@/services/notificationService';

interface NotificationCenterProps {
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

export const NotificationCenter: React.FC<NotificationCenterProps> = ({ isOpen, onClose }) => {
  const { 
    notifications, 
    markAsRead, 
    markAllAsRead, 
    clearNotification, 
    clearAllNotifications,
    unreadCount 
  } = useNotifications();

  const notificationService = NotificationService.getInstance();
  
  // UI State
  const [activeTab, setActiveTab] = useState<'notifications' | 'analytics'>('notifications');
  const [searchTerm, setSearchTerm] = useState('');
  const [showFilters, setShowFilters] = useState(false);
  const [selectedNotifications, setSelectedNotifications] = useState<Set<string>>(new Set());
  
  // Filter state
  const [filter, setFilter] = useState<NotificationFilter>({
    readStatus: 'all',
  });

  // Analytics
  const analytics = useMemo(() => 
    notificationService.generateAnalytics(notifications),
    [notifications]
  );

  // Filtered notifications
  const filteredNotifications = useMemo(() => 
    notificationService.filterNotifications(notifications, filter),
    [notifications, filter]
  );

  // Grouped notifications
  const groupedNotifications = useMemo(() => {
    const groups: Record<string, BaseNotification[]> = {};
    
    filteredNotifications.forEach(notification => {
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
  }, [filteredNotifications]);

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

  const handleSelectNotification = (notificationId: string, event: React.MouseEvent) => {
    event.stopPropagation();
    setSelectedNotifications(prev => {
      const newSet = new Set(prev);
      if (newSet.has(notificationId)) {
        newSet.delete(notificationId);
      } else {
        newSet.add(notificationId);
      }
      return newSet;
    });
  };

  const handleSelectAll = () => {
    if (selectedNotifications.size === filteredNotifications.length) {
      setSelectedNotifications(new Set());
    } else {
      setSelectedNotifications(new Set(filteredNotifications.map(n => n.id)));
    }
  };

  const handleBatchMarkAsRead = () => {
    selectedNotifications.forEach(id => markAsRead(id));
    setSelectedNotifications(new Set());
  };

  const handleBatchDelete = () => {
    selectedNotifications.forEach(id => clearNotification(id));
    setSelectedNotifications(new Set());
  };

  const handleExport = (format: 'json' | 'csv') => {
    const exportData = notificationService.exportNotifications(filteredNotifications, format);
    const blob = new Blob([exportData], { 
      type: format === 'json' ? 'application/json' : 'text/csv' 
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `notifications.${format}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const updateFilter = (updates: Partial<NotificationFilter>) => {
    setFilter(prev => ({ ...prev, ...updates }));
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
          className="absolute right-0 top-0 h-full w-full max-w-6xl bg-white dark:bg-gray-900 shadow-xl"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="border-b border-gray-200 dark:border-gray-700 p-6">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center space-x-3">
                <Bell className="h-6 w-6 text-muted-foreground dark:text-muted-foreground" />
                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
                  Notification Center
                </h1>
                {unreadCount > 0 && (
                  <span className="bg-red-500 text-white text-sm px-3 py-1 rounded-full">
                    {unreadCount} unread
                  </span>
                )}
              </div>
              <button
                onClick={onClose}
                className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
              >
                <X className="h-6 w-6" />
              </button>
            </div>

            {/* Tabs */}
            <div className="flex space-x-1">
              <button
                onClick={() => setActiveTab('notifications')}
                className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                  activeTab === 'notifications'
                    ? 'bg-blue-500 text-white'
                    : 'text-muted-foreground dark:text-muted-foreground hover:bg-gray-100 dark:hover:bg-gray-800'
                }`}
              >
                <div className="flex items-center space-x-2">
                  <Bell className="h-4 w-4" />
                  <span>Notifications</span>
                </div>
              </button>
              <button
                onClick={() => setActiveTab('analytics')}
                className={`px-4 py-2 rounded-lg font-medium transition-colors ${
                  activeTab === 'analytics'
                    ? 'bg-blue-500 text-white'
                    : 'text-muted-foreground dark:text-muted-foreground hover:bg-gray-100 dark:hover:bg-gray-800'
                }`}
              >
                <div className="flex items-center space-x-2">
                  <BarChart3 className="h-4 w-4" />
                  <span>Analytics</span>
                </div>
              </button>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-hidden">
            {activeTab === 'notifications' ? (
              <div className="h-full flex">
                {/* Main Content */}
                <div className="flex-1 overflow-y-auto">
                  {/* Search and Filters */}
                  <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                    <div className="flex items-center space-x-4 mb-4">
                      <div className="flex-1 relative">
                        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                        <input
                          type="text"
                          placeholder="Search notifications..."
                          value={searchTerm}
                          onChange={(e) => {
                            setSearchTerm(e.target.value);
                            updateFilter({ searchTerm: e.target.value });
                          }}
                          className="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-500 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        />
                      </div>
                      
                      <button
                        onClick={() => setShowFilters(!showFilters)}
                        className={`px-4 py-2 rounded-lg transition-colors flex items-center space-x-2 ${
                          showFilters
                            ? 'bg-blue-500 text-white'
                            : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300'
                        }`}
                      >
                        <Filter className="h-4 w-4" />
                        <span>Filters</span>
                        {showFilters ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                      </button>

                      <div className="flex items-center space-x-2">
                        <button
                          onClick={() => handleExport('json')}
                          className="p-2 text-muted-foreground dark:text-muted-foreground hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                          title="Export as JSON"
                        >
                          <Download className="h-4 w-4" />
                        </button>
                        <button
                          onClick={() => handleExport('csv')}
                          className="p-2 text-muted-foreground dark:text-muted-foreground hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
                          title="Export as CSV"
                        >
                          <Download className="h-4 w-4" />
                        </button>
                      </div>
                    </div>

                    {/* Expanded Filters */}
                    <AnimatePresence>
                      {showFilters && (
                        <motion.div
                          initial={{ height: 0, opacity: 0 }}
                          animate={{ height: 'auto', opacity: 1 }}
                          exit={{ height: 0, opacity: 0 }}
                          className="space-y-4"
                        >
                          <div className="grid grid-cols-3 gap-4">
                            <div>
                              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Read Status
                              </label>
                              <select
                                value={filter.readStatus}
                                onChange={(e) => updateFilter({ readStatus: e.target.value as any })}
                                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                              >
                                <option value="all">All</option>
                                <option value="read">Read</option>
                                <option value="unread">Unread</option>
                              </select>
                            </div>

                            <div>
                              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Types
                              </label>
                              <div className="space-y-2">
                                {(['success', 'error', 'warning', 'info'] as NotificationType[]).map(type => (
                                  <label key={type} className="flex items-center space-x-2">
                                    <input
                                      type="checkbox"
                                      checked={filter.types?.includes(type) || false}
                                      onChange={(e) => {
                                        const currentTypes = filter.types || [];
                                        if (e.target.checked) {
                                          updateFilter({ types: [...currentTypes, type] });
                                        } else {
                                          updateFilter({ types: currentTypes.filter(t => t !== type) });
                                        }
                                      }}
                                      className="rounded"
                                    />
                                    <span className="text-sm capitalize">{type}</span>
                                  </label>
                                ))}
                              </div>
                            </div>

                            <div>
                              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Priority
                              </label>
                              <div className="space-y-2">
                                {(['low', 'medium', 'high', 'critical'] as NotificationPriority[]).map(priority => (
                                  <label key={priority} className="flex items-center space-x-2">
                                    <input
                                      type="checkbox"
                                      checked={filter.priorities?.includes(priority) || false}
                                      onChange={(e) => {
                                        const currentPriorities = filter.priorities || [];
                                        if (e.target.checked) {
                                          updateFilter({ priorities: [...currentPriorities, priority] });
                                        } else {
                                          updateFilter({ priorities: currentPriorities.filter(p => p !== priority) });
                                        }
                                      }}
                                      className="rounded"
                                    />
                                    <span className="text-sm capitalize">{priority}</span>
                                  </label>
                                ))}
                              </div>
                            </div>
                          </div>
                        </motion.div>
                      )}
                    </AnimatePresence>

                    {/* Batch Actions */}
                    {selectedNotifications.size > 0 && (
                      <div className="mt-4 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg flex items-center justify-between">
                        <span className="text-sm text-blue-700 dark:text-blue-300">
                          {selectedNotifications.size} selected
                        </span>
                        <div className="flex items-center space-x-2">
                          <button
                            onClick={handleBatchMarkAsRead}
                            className="px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
                          >
                            Mark as Read
                          </button>
                          <button
                            onClick={handleBatchDelete}
                            className="px-3 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600 transition-colors"
                          >
                            Delete
                          </button>
                        </div>
                      </div>
                    )}

                    {/* Quick Actions */}
                    <div className="mt-4 flex items-center justify-between">
                      <div className="flex items-center space-x-2">
                        <input
                          type="checkbox"
                          checked={selectedNotifications.size === filteredNotifications.length && filteredNotifications.length > 0}
                          onChange={handleSelectAll}
                          className="rounded"
                        />
                        <span className="text-sm text-muted-foreground dark:text-muted-foreground">Select all</span>
                      </div>
                      <div className="flex items-center space-x-2">
                        <button
                          onClick={markAllAsRead}
                          disabled={unreadCount === 0}
                          className="px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        >
                          Mark All Read
                        </button>
                        <button
                          onClick={clearAllNotifications}
                          className="px-3 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600 transition-colors"
                        >
                          Clear All
                        </button>
                      </div>
                    </div>
                  </div>

                  {/* Notifications List */}
                  <div className="p-6">
                    {Object.keys(groupedNotifications).length === 0 ? (
                      <div className="flex flex-col items-center justify-center h-64 text-muted-foreground dark:text-muted-foreground">
                        <BellOff className="h-12 w-12 mb-4" />
                        <p className="text-lg font-medium">No notifications</p>
                        <p className="text-sm">You&apos;re all caught up!</p>
                      </div>
                    ) : (
                      <div className="space-y-6">
                        {Object.entries(groupedNotifications).map(([groupKey, groupNotifications]) => (
                          <div key={groupKey}>
                            <h3 className="text-sm font-medium text-muted-foreground dark:text-muted-foreground mb-3 flex items-center">
                              <Calendar className="h-4 w-4 mr-2" />
                              {groupKey}
                            </h3>
                            <div className="space-y-2">
                              {groupNotifications.map((notification) => {
                                const Icon = NOTIFICATION_ICONS[notification.type];
                                const isSelected = selectedNotifications.has(notification.id);
                                return (
                                  <motion.div
                                    key={notification.id}
                                    layout
                                    initial={{ opacity: 0, y: 20 }}
                                    animate={{ opacity: 1, y: 0 }}
                                    exit={{ opacity: 0, y: -20 }}
                                    className={`
                                      p-4 rounded-lg border cursor-pointer transition-all hover:shadow-md
                                      ${notification.read 
                                        ? 'bg-gray-50 dark:bg-gray-800 border-gray-200 dark:border-gray-700' 
                                        : 'bg-white dark:bg-gray-900 border-blue-200 dark:border-blue-800 shadow-sm'
                                      }
                                      ${isSelected ? 'ring-2 ring-blue-500' : ''}
                                    `}
                                    onClick={() => handleNotificationClick(notification)}
                                  >
                                    <div className="flex items-start space-x-3">
                                      <input
                                        type="checkbox"
                                        checked={isSelected}
                                        onChange={(e) => handleSelectNotification(notification.id, e)}
                                        onClick={(e) => e.stopPropagation()}
                                        className="mt-1 rounded"
                                      />
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
                                              <Trash2 className="h-3 w-3 text-muted-foreground" />
                                            </button>
                                          </div>
                                        </div>
                                        <p className={`text-sm mt-1 ${
                                          notification.read 
                                            ? 'text-muted-foreground dark:text-muted-foreground' 
                                            : 'text-gray-700 dark:text-gray-300'
                                        }`}>
                                          {notification.message}
                                        </p>
                                        <p className="text-xs text-muted-foreground mt-2">
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
                </div>
              </div>
            ) : (
              <AnalyticsTab analytics={analytics} />
            )}
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
};

// Analytics Tab Component
const AnalyticsTab: React.FC<{ analytics: NotificationAnalytics }> = ({ analytics }) => {
  return (
    <div className="p-6">
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
        <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground dark:text-muted-foreground">Total Notifications</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">{analytics.totalNotifications}</p>
            </div>
            <Bell className="h-8 w-8 text-blue-500" />
          </div>
        </div>

        <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground dark:text-muted-foreground">Unread</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">{analytics.unreadCount}</p>
            </div>
            <AlertCircle className="h-8 w-8 text-red-500" />
          </div>
        </div>

        <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground dark:text-muted-foreground">Read Rate</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {analytics.totalNotifications > 0 
                  ? Math.round(((analytics.totalNotifications - analytics.unreadCount) / analytics.totalNotifications) * 100)
                  : 0}%
              </p>
            </div>
            <CheckCircle className="h-8 w-8 text-green-500" />
          </div>
        </div>

        <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-muted-foreground dark:text-muted-foreground">Avg Response Time</p>
              <p className="text-2xl font-bold text-gray-900 dark:text-white">
                {analytics.averageResponseTime ? `${Math.round(analytics.averageResponseTime)}m` : 'N/A'}
              </p>
            </div>
            <Clock className="h-8 w-8 text-purple-500" />
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Type Distribution */}
        <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Type Distribution</h3>
          <div className="space-y-3">
            {Object.entries(analytics.typeDistribution).map(([type, count]) => {
              const Icon = NOTIFICATION_ICONS[type as NotificationType];
              const percentage = analytics.totalNotifications > 0 ? (count / analytics.totalNotifications) * 100 : 0;
              return (
                <div key={type} className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <Icon className={`h-4 w-4 ${ICON_COLORS[type as NotificationType]}`} />
                    <span className="text-sm capitalize text-gray-700 dark:text-gray-300">{type}</span>
                  </div>
                  <div className="flex items-center space-x-2">
                    <div className="w-20 bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                      <div 
                        className="bg-blue-500 h-2 rounded-full"
                        style={{ width: `${percentage}%` }}
                      />
                    </div>
                    <span className="text-sm text-muted-foreground dark:text-muted-foreground w-8">{count}</span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Priority Distribution */}
        <div className="bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Priority Distribution</h3>
          <div className="space-y-3">
            {Object.entries(analytics.priorityDistribution).map(([priority, count]) => {
              const percentage = analytics.totalNotifications > 0 ? (count / analytics.totalNotifications) * 100 : 0;
              return (
                <div key={priority} className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <span className={`px-2 py-1 text-xs rounded-full ${PRIORITY_BADGES[priority as NotificationPriority].className}`}>
                      {PRIORITY_BADGES[priority as NotificationPriority].label}
                    </span>
                    <span className="text-sm capitalize text-gray-700 dark:text-gray-300">{priority}</span>
                  </div>
                  <div className="flex items-center space-x-2">
                    <div className="w-20 bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                      <div 
                        className="bg-orange-500 h-2 rounded-full"
                        style={{ width: `${percentage}%` }}
                      />
                    </div>
                    <span className="text-sm text-muted-foreground dark:text-muted-foreground w-8">{count}</span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
};
