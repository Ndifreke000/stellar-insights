import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Bell, CheckSquare, Filter, Wifi, WifiOff, X } from 'lucide-react';
import useEnhancedNotificationCenter from './useEnhancedNotificationCenter';
import { BaseNotification } from '@/types/notifications';
import { Dispatch, SetStateAction } from 'react';

const NotificationHeader = ({ unreadCount,
  markAllAsRead,
  isWebSocketConnected,
  filteredNotifications,
  showFilters,
  setShowFilters, onClose }: {
    unreadCount:number,
    onClose: () => void,
    markAllAsRead:() => void,
    isWebSocketConnected:boolean,
    filteredNotifications: BaseNotification[],
    showFilters: boolean,
    setShowFilters: Dispatch<SetStateAction<boolean>>
  }) => {
 
  return (
    <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-slate-700">
      <div className="flex items-center gap-3">
        <div className="relative">
          <Bell className="h-6 w-6 text-blue-600 dark:text-blue-400" />
          {unreadCount > 0 && (
            <Badge className="absolute -top-2 -right-2 bg-red-500 text-white text-xs min-w-[20px] h-5">
              {unreadCount > 99 ? '99+' : unreadCount}
            </Badge>
          )}
        </div>
        <div>
          <h2 className="text-xl font-bold text-gray-900 dark:text-white">
            Notification Center
          </h2>
          <div className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
            <span>
              {isWebSocketConnected ? (
                <>
                  <Wifi className="h-3 w-3 text-green-500" />
                  Connected
                </>
              ) : (
                <>
                  <WifiOff className="h-3 w-3 text-red-500" />
                  Disconnected
                </>
              )}
            </span>
            <span>•</span>
            <span>{filteredNotifications.length} notifications</span>
            {unreadCount > 0 && (
              <>
                <span>•</span>
                <span>{unreadCount} unread</span>
              </>
            )}
          </div>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowFilters(!showFilters)}
            >
              <Filter className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Toggle Filters</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="sm" onClick={markAllAsRead}>
              <CheckSquare className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Mark All as Read</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="sm" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Close</TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
};

export default NotificationHeader;
