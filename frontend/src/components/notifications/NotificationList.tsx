'use client';

import React, { useState, useCallback } from 'react';
import { Bell, BellOff, X, CheckCircle, AlertCircle, AlertTriangle, Info, ExternalLink, MoreVertical, Eye, Trash2, Copy } from 'lucide-react';
import { BaseNotification, NotificationType, NotificationPriority } from '@/types/notifications';
import { useNotifications } from '@/contexts/NotificationContext';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from '@/components/ui/dropdown-menu';

interface NotificationItemProps {
  notification: BaseNotification;
  onSelect?: (notification: BaseNotification) => void;
  showActions?: boolean;
}

const NOTIFICATION_ICONS = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const TYPE_COLORS: {
  success: string;
  error: string;
  warning: string;
  info: string;
} = {
  success: 'text-green-500',
  error: 'text-red-500',
  warning: 'text-yellow-500',
  info: 'text-blue-500',
};

const PRIORITY_COLORS: {
  low: string;
  medium: string;
  high: string;
  critical: string;
} = {
  low: 'border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-900',
  medium: 'border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-900/20',
  high: 'border-orange-200 bg-orange-50 dark:border-orange-800 dark:bg-orange-900/20',
  critical: 'border-red-200 bg-red-50 dark:border-red-800 dark:bg-red-900/20',
};

export const NotificationItem: React.FC<NotificationItemProps> = ({
  notification,
  onSelect,
  showActions = true,
}) => {
  const { markAsRead, clearNotification } = useNotifications();
  const [isExpanded, setIsExpanded] = useState(false);

  const handleMarkAsRead = useCallback(() => {
    markAsRead(notification.id);
  }, [markAsRead, notification.id]);

  const handleDelete = useCallback(() => {
    clearNotification(notification.id);
  }, [clearNotification, notification.id]);

  const handleCopy = useCallback(() => {
    const text = `${notification.title}\n\n${notification.message}\n\n${new Date(notification.timestamp).toLocaleString()}`;
    navigator.clipboard.writeText(text);
  }, [notification]);

  const handleSelect = useCallback(() => {
    if (onSelect) {
      onSelect(notification);
    }
    if (!notification.read) {
      handleMarkAsRead();
    }
  }, [onSelect, notification, handleMarkAsRead]);

  const IconComponent = NOTIFICATION_ICONS[notification.type];
  const priorityColor = PRIORITY_COLORS[notification.priority];
  const typeColor = TYPE_COLORS[notification.type];

  return (
    <div
      className={`
        relative p-4 rounded-lg border transition-all cursor-pointer
        ${notification.read 
          ? 'bg-white dark:bg-slate-900 border-gray-200 dark:border-slate-700' 
          : 'bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800'
        }
        hover:shadow-md hover:border-blue-300 dark:hover:border-blue-600
      `}
      onClick={handleSelect}
    >
      {/* Priority Indicator */}
      {notification.priority === 'critical' && (
        <div className="absolute -left-1 top-0 bottom-0 w-1 bg-red-500 rounded-l-lg" />
      )}

      <div className="flex items-start gap-3">
        {/* Icon */}
        <div className="shrink-0 mt-0.5">
          {React.createElement(IconComponent, { 
            className: `h-5 w-5 ${typeColor}` 
          })}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          {/* Header */}
          <div className="flex items-center gap-2 mb-2">
            <h4 className="font-semibold text-gray-900 dark:text-white truncate">
              {notification.title}
            </h4>
            <Badge variant="secondary" className="text-xs">
              {notification.priority}
            </Badge>
            <Badge variant="outline" className="text-xs">
              {notification.category}
            </Badge>
            {!notification.read && (
              <div className="w-2 h-2 bg-blue-500 rounded-full" />
            )}
          </div>
          
          <p className="text-sm text-gray-600 dark:text-gray-400 line-clamp-2">
            {notification.message}
          </p>
          
          <div className="flex items-center gap-4 mt-2 text-xs text-gray-500">
            <span>
              {new Date(notification.timestamp).toLocaleString()}
            </span>
            {notification.metadata?.source && (
              <span>
                Source: {notification.metadata.source as string}
              </span>
            )}
          </div>
        </div>

        {/* Actions */}
        {showActions && (
          <div className="flex items-center gap-1">
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button 
                  variant="ghost" 
                  size="sm" 
                  onClick={(e) => e.stopPropagation()}
                >
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={(e) => {
                  e.stopPropagation();
                  handleMarkAsRead();
                }}>
                  <Eye className="h-4 w-4 mr-2" />
                  Mark as Read
                </DropdownMenuItem>
                <DropdownMenuItem onClick={(e) => {
                  e.stopPropagation();
                  handleCopy();
                }}>
                  <Copy className="h-4 w-4 mr-2" />
                  Copy
                </DropdownMenuItem>
                {notification.metadata?.url && (
                  <DropdownMenuItem onClick={(e) => {
                    e.stopPropagation();
                    window.open(notification.metadata.url as string, '_blank');
                  }}>
                    <ExternalLink className="h-4 w-4 mr-2" />
                    Open Link
                  </DropdownMenuItem>
                )}
                <DropdownMenuSeparator />
                {notification.actions?.map(action => (
                  <DropdownMenuItem
                    key={action.id}
                    onClick={(e) => {
                      e.stopPropagation();
                      action.onClick();
                    }}
                  >
                    {action.label}
                  </DropdownMenuItem>
                ))}
                <DropdownMenuSeparator />
                <DropdownMenuItem 
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDelete();
                  }}
                  className="text-red-600"
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        )}
      </div>
    </div>
  );
};

interface NotificationListProps {
  notifications: BaseNotification[];
  onNotificationSelect?: (notification: BaseNotification) => void;
  emptyMessage?: string;
  emptyDescription?: string;
}

export const NotificationList: React.FC<NotificationListProps> = ({
  notifications,
  onNotificationSelect,
  emptyMessage = 'No notifications found',
  emptyDescription = 'You\'re all caught up!',
}) => {
  if (notifications.length === 0) {
    return (
      <div className="text-center py-12">
        <BellOff className="h-12 w-12 mx-auto mb-4 text-gray-400" />
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
          {emptyMessage}
        </h3>
        <p className="text-gray-600 dark:text-gray-400">
          {emptyDescription}
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {notifications.map((notification) => (
        <NotificationItem
          key={notification.id}
          notification={notification}
          onSelect={onNotificationSelect}
        />
      ))}
    </div>
  );
};
