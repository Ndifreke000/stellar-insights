import { useNotifications } from "@/contexts/NotificationContext";
import {NotificationFilter } from "@/services/notificationService";
import { BaseNotification, NotificationAction, NotificationPreferences } from "@/types/notifications";
import { format, isToday, isWithinInterval, isYesterday, subDays } from "date-fns";
import { Dispatch, SetStateAction, useCallback, useMemo, useState } from "react";

export interface IUseEnhancedNotificationCenter {
  notifications: BaseNotification[];
  unreadCount: number;
  markAsRead: (id: string) => void;
  markAllAsRead: () => void;
  clearNotification: (id: string) => void;
  clearAllNotifications: () => void;
  preferences: NotificationPreferences;
  updatePreferences: (preferences: Partial<NotificationPreferences>) => void;
  isWebSocketConnected: boolean;
  filteredNotifications: BaseNotification[];
  groupedNotifications: Record<string, BaseNotification[]>;
  handleSelectAll: () => void;
  handleBulkMarkAsRead: () => void;
  handleBulkDelete: () => void;
  handleBulkExport: () => void;
  handleNotificationAction: (
    notification: BaseNotification,
    action: NotificationAction,
  ) => void;
  copyNotificationText: (notification: BaseNotification) => void;
  shareNotification: (notification: BaseNotification) => Promise<void>;
  searchTerm: string;
  setSearchTerm: Dispatch<SetStateAction<string>>;
  selectedFilters: NotificationFilter;
  setSelectedFilters: Dispatch<SetStateAction<NotificationFilter>>;
  selectedNotifications: Set<string>;
  setSelectedNotifications: Dispatch<SetStateAction<Set<string>>>;
  showFilters: boolean;
  setShowFilters: Dispatch<SetStateAction<boolean>>;
  groupBy: 'priority' | 'type' | 'none' | 'date';
  setGroupBy: Dispatch<SetStateAction<'priority' | 'type' | 'none' | 'date'>>;
  sortBy: 'priority' | 'type' | 'timestamp';
  setSortBy: Dispatch<SetStateAction<'priority' | 'type' | 'timestamp'>>;
  sortOrder: 'asc' | 'desc';
  setSortOrder: Dispatch<SetStateAction<'asc' | 'desc'>>;
  viewMode: "list" | "grid" | "compact"; setViewMode:Dispatch<SetStateAction<"list" | "grid" | "compact">>;
}


const useEnhancedNotificationCenter = () => {
  const {
    notifications,
    unreadCount,
    markAsRead,
    markAllAsRead,
    clearNotification,
    clearAllNotifications,
    preferences,
    updatePreferences,
    isWebSocketConnected,
  } = useNotifications();

  const [searchTerm, setSearchTerm] = useState('');
  const [selectedFilters, setSelectedFilters] = useState<NotificationFilter>({});
  const [selectedNotifications, setSelectedNotifications] = useState<Set<string>>(new Set());
  const [showFilters, setShowFilters] = useState(false);
  const [groupBy, setGroupBy] = useState<'none' | 'date' | 'type' | 'priority'>('date');
  const [sortBy, setSortBy] = useState<'timestamp' | 'priority' | 'type'>('timestamp');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [viewMode, setViewMode] = useState<'list' | 'grid' | 'compact'>('list');

  // Calculate analytics


  // Filter and sort notifications
  const filteredNotifications = useMemo(() => {
    let filtered = notifications.filter(notification => {
      // Search filter
      if (searchTerm) {
        const searchLower = searchTerm.toLowerCase();
        if (!notification.title.toLowerCase().includes(searchLower) &&
          !notification.message.toLowerCase().includes(searchLower)) {
          return false;
        }
      }

      // Type filter
      if (selectedFilters.types && selectedFilters.types.length > 0) {
        if (!selectedFilters.types.includes(notification.type)) {
          return false;
        }
      }

      // Priority filter
      if (selectedFilters.priorities && selectedFilters.priorities.length > 0) {
        if (!selectedFilters.priorities.includes(notification.priority)) {
          return false;
        }
      }

      // Category filter
      if (selectedFilters.categories && selectedFilters.categories.length > 0) {
        if (!selectedFilters.categories.includes(notification.category)) {
          return false;
        }
      }

      // Read status filter
      if (selectedFilters.readStatus) {
        if (selectedFilters.readStatus === 'read' && !notification.read) {
          return false;
        }
        if (selectedFilters.readStatus === 'unread' && notification.read) {
          return false;
        }
      }

      // Date range filter
      if (selectedFilters.dateRange) {
        if (!isWithinInterval(notification.timestamp, {
          start: selectedFilters.dateRange.start,
          end: selectedFilters.dateRange.end,
        })) {
          return false;
        }
      }

      return true;
    });

    // Sort notifications
    filtered.sort((a, b) => {
      let comparison = 0;

      switch (sortBy) {
        case 'timestamp':
          comparison = a.timestamp.getTime() - b.timestamp.getTime();
          break;
        case 'priority':
          const priorityOrder = { low: 0, medium: 1, high: 2, critical: 3 };
          comparison = priorityOrder[a.priority] - priorityOrder[b.priority];
          break;
        case 'type':
          comparison = a.type.localeCompare(b.type);
          break;
      }

      return sortOrder === 'asc' ? comparison : -comparison;
    });

    return filtered;
  }, [notifications, searchTerm, selectedFilters, sortBy, sortOrder]);

  // Group notifications
  const groupedNotifications = useMemo(() => {
    if (groupBy === 'none') {
      return { '': filteredNotifications };
    }

    const groups: Record<string, BaseNotification[]> = {};

    filteredNotifications.forEach(notification => {
      let key = '';

      switch (groupBy) {
        case 'date':
          if (isToday(notification.timestamp)) {
            key = 'Today';
          } else if (isYesterday(notification.timestamp)) {
            key = 'Yesterday';
          } else if (notification.timestamp > subDays(new Date(), 7)) {
            key = 'Last 7 Days';
          } else if (notification.timestamp > subDays(new Date(), 30)) {
            key = 'Last 30 Days';
          } else {
            key = 'Older';
          }
          break;
        case 'type':
          key = notification.type.charAt(0).toUpperCase() + notification.type.slice(1);
          break;
        case 'priority':
          key = notification.priority.charAt(0).toUpperCase() + notification.priority.slice(1);
          break;
      }

      if (!groups[key]) {
        groups[key] = [];
      }
      groups[key].push(notification);
    });

    return groups;
  }, [filteredNotifications, groupBy]);

  // Bulk actions
  const handleSelectAll = useCallback(() => {
    if (selectedNotifications.size === filteredNotifications.length) {
      setSelectedNotifications(new Set());
    } else {
      setSelectedNotifications(new Set(filteredNotifications.map(n => n.id)));
    }
  }, [filteredNotifications, selectedNotifications.size]);

  const handleBulkMarkAsRead = useCallback(() => {
    selectedNotifications.forEach(id => markAsRead(id));
    setSelectedNotifications(new Set());
  }, [selectedNotifications, markAsRead]);

  const handleBulkDelete = useCallback(() => {
    selectedNotifications.forEach(id => clearNotification(id));
    setSelectedNotifications(new Set());
  }, [selectedNotifications, clearNotification]);

  const handleBulkExport = useCallback(() => {
    const exportData = filteredNotifications.filter(n => selectedNotifications.has(n.id));
    const dataStr = JSON.stringify(exportData, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `notifications-${format(new Date(), 'yyyy-MM-dd-HH-mm-ss')}.json`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  }, [filteredNotifications, selectedNotifications]);

  const handleNotificationAction = useCallback((notification: BaseNotification, action: NotificationAction) => {
    action.onClick();
    if (action.id.includes('mark-read')) {
      markAsRead(notification.id);
    }
  }, [markAsRead]);

  const copyNotificationText = useCallback((notification: BaseNotification) => {
    const text = `${notification.title}\n\n${notification.message}\n\n${format(notification.timestamp, 'PPpp')}`;
    navigator.clipboard.writeText(text);
  }, []);

  const shareNotification = useCallback(async (notification: BaseNotification) => {
    if (navigator.share) {
      try {
        await navigator.share({
          title: notification.title,
          text: notification.message,
        });
      } catch (error) {
        console.error('Error sharing notification:', error);
      }
    } else {
      copyNotificationText(notification);
    }
  }, [copyNotificationText]);

  return { notifications, unreadCount, markAsRead, markAllAsRead, clearNotification, clearAllNotifications, preferences, updatePreferences, isWebSocketConnected, filteredNotifications, groupedNotifications, handleSelectAll, handleBulkMarkAsRead, handleBulkDelete, handleBulkExport, handleNotificationAction, copyNotificationText, shareNotification, searchTerm, setSearchTerm, selectedFilters, setSelectedFilters, selectedNotifications, setSelectedNotifications, showFilters, setShowFilters, groupBy, setGroupBy, sortBy, setSortBy, sortOrder, setSortOrder, viewMode, setViewMode };
}

export default useEnhancedNotificationCenter;