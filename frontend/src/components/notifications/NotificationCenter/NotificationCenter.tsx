'use client';

import React, { useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Bell,
  BellOff,
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
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { format, isToday, isYesterday, subDays, startOfDay, endOfDay } from 'date-fns';
import { BaseNotification, NotificationType, NotificationPriority } from '@/types/notifications';
import { useNotifications } from '@/contexts/NotificationContext';
import { NotificationService, NotificationFilter, NotificationAnalytics } from '@/services/notificationService';
import AnalyticsTab from './AnalyticsTab';
import { ICON_COLORS, NOTIFICATION_ICONS, PRIORITY_BADGES } from './helpers';
import NotificationsTab from './NotificationsTab';

interface NotificationCenterProps {
  isOpen: boolean;
  onClose: () => void;
}

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
              <NotificationsTab
                searchTerm={searchTerm}
                setSearchTerm={setSearchTerm}
                updateFilter={updateFilter}
                setShowFilters={setShowFilters}
                showFilters={showFilters}
                filter={filter}
                handleSelectAll={handleSelectAll}
                handleBatchMarkAsRead={handleBatchMarkAsRead}
                handleBatchDelete={handleBatchDelete}
                handleExport={handleExport}
                markAllAsRead={markAllAsRead}
                clearAllNotifications={clearAllNotifications}
                selectedNotifications={selectedNotifications}
                groupedNotifications={groupedNotifications}
                unreadCount={unreadCount}
                handleNotificationClick={handleNotificationClick}
                handleSelectNotification={handleSelectNotification}
                filteredNotifications={filteredNotifications}
                clearNotification={clearNotification}
              />
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

