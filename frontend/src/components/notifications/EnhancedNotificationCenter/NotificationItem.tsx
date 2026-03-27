import React, { useCallback } from 'react';
import { motion } from 'framer-motion';
import {
  MoreVertical,
  Eye,
  Copy,
  Share2,
  Trash2,
  Clock,
  Globe,
} from 'lucide-react';
import { format } from 'date-fns';
import { BaseNotification, NotificationAction } from '@/types/notifications';
import { Checkbox } from '@/components/ui/checkbox';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu';
import { NOTIFICATION_ICONS, PRIORITY_COLORS, TYPE_COLORS } from './Constants';
import { cn } from '@/lib/utils';

interface ItemProps {
  notification: BaseNotification;
  viewMode: 'list' | 'grid' | 'compact';
  selectedNotifications: Set<string>;
  setSelectedNotifications: (selected: Set<string>) => void;
  markAsRead: (id: string) => void;
  clearNotification: (id: string) => void;
  copyNotificationText: (notification: BaseNotification) => void;
  shareNotification: (notification: BaseNotification) => void;
  handleNotificationAction: (
    notification: BaseNotification,
    action: NotificationAction,
  ) => void;
}

export const NotificationItem: React.FC<ItemProps> = ({
  notification,
  selectedNotifications,
  setSelectedNotifications,
  markAsRead,
  clearNotification,
  copyNotificationText,
  shareNotification,
  handleNotificationAction,
}) => {
  const Icon = NOTIFICATION_ICONS[notification.type];
  return (
    <motion.div
      key={notification.id}
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className={cn(
        `relative p-4 rounded-lg border transition-all cursor-pointer hover:shadow-md hover:border-blue-300 dark:hover:border-blue-600`,
        notification.read
          ? 'bg-white dark:bg-slate-900 border-gray-200 dark:border-slate-700'
          : 'bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800',
      )}
      onClick={() => {
        if (!notification.read) {
          markAsRead(notification.id);
        }
      }}
    >
      {/* Selection Checkbox */}
      <div className="absolute top-3 left-3">
        <Checkbox
          checked={selectedNotifications.has(notification.id)}
          onCheckedChange={(checked) => {
            const newSelected = new Set(selectedNotifications);
            if (checked) {
              newSelected.add(notification.id);
            } else {
              newSelected.delete(notification.id);
            }
            setSelectedNotifications(newSelected);
          }}
          onClick={(e) => e.stopPropagation()}
        />
      </div>

      {/* Notification Content */}
      <div className="pl-8">
        <div className="flex items-start justify-between gap-3">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-2">
              <Icon className={`h-4 w-4 ${TYPE_COLORS[notification.type]}`} />
              <Badge className={PRIORITY_COLORS[notification.priority]}>
                {notification.priority}
              </Badge>
              <Badge variant="outline" className="text-xs">
                {notification.category}
              </Badge>
              {!notification.read && (
                <div className="w-2 h-2 bg-blue-500 rounded-full" />
              )}
            </div>

            <h4 className="font-semibold text-gray-900 dark:text-white truncate">
              {notification.title}
            </h4>
            <p className="text-sm text-gray-600 dark:text-gray-400 line-clamp-2">
              {notification.message}
            </p>

            <div className="flex items-center gap-4 mt-2 text-xs text-gray-500">
              <span className="flex items-center gap-1">
                <Clock className="h-3 w-3" />
                {format(notification.timestamp, 'PPp')}
              </span>
              {notification.metadata?.source && (
                <span className="flex items-center gap-1">
                  <Globe className="h-3 w-3" />
                  {/* {notification.metadata.source} */}
                </span>
              )}
            </div>
          </div>

          {/* Actions */}
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
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    markAsRead(notification.id);
                  }}
                >
                  <Eye className="h-4 w-4 mr-2" />
                  Mark as Read
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    copyNotificationText(notification);
                  }}
                >
                  <Copy className="h-4 w-4 mr-2" />
                  Copy
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    shareNotification(notification);
                  }}
                >
                  <Share2 className="h-4 w-4 mr-2" />
                  Share
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                {notification.actions?.map((action) => (
                  <DropdownMenuItem
                    key={action.id}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleNotificationAction(notification, action);
                    }}
                  >
                    {action.label}
                  </DropdownMenuItem>
                ))}
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    clearNotification(notification.id);
                  }}
                  className="text-red-600"
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </div>
    </motion.div>
  );
};
