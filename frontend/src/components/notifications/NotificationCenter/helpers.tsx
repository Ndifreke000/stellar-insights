import { NotificationFilter } from "@/services/notificationService";
import { BaseNotification, NotificationPriority, NotificationType } from "@/types/notifications";
import { format, isToday, isYesterday } from "date-fns";
import { AlertCircle, AlertTriangle, CheckCircle, Info } from "lucide-react";
import React from "react";

export interface NotificationCenterProps {
  searchTerm: string,
  setSearchTerm: React.Dispatch<React.SetStateAction<string>>,
  updateFilter: (updates: Partial<NotificationFilter>) => void,
  setShowFilters : React.Dispatch<React.SetStateAction<boolean>>,
  showFilters: boolean,
  filter: NotificationFilter,
  handleSelectAll: () => void,
  handleBatchMarkAsRead: () => void,
  handleBatchDelete: () => void,
  handleExport,
  markAllAsRead:() => void,
  clearAllNotifications:() => void,
    selectedNotifications: Set<string>;
  groupedNotifications:Record<string, BaseNotification[]>;
  unreadCount: number,
  handleNotificationClick: (notification: BaseNotification) => void,
  handleSelectNotification: (id: string, event: React.MouseEvent) => void,
  filteredNotifications: BaseNotification[],
  clearNotification:(id: string) => void,
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
  const formatTime = (date: Date) => {
    if (isToday(date)) {
      return format(date, 'HH:mm');
    } else if (isYesterday(date)) {
      return `Yesterday ${format(date, 'HH:mm')}`;
    } else {
      return format(date, 'MMM dd, HH:mm');
    }
  };
export {
  NOTIFICATION_ICONS, ICON_COLORS, PRIORITY_BADGES,formatTime
}