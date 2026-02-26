'use client';

import React, { useState, useMemo, useCallback } from 'react';
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
  RefreshCw,
  Archive,
  Star,
  MoreVertical,
  Eye,
  EyeOff,
  Tag,
  Zap,
  Shield,
  Globe,
  Database,
  Cpu,
  Wifi,
  WifiOff,
  CheckSquare,
  Square,
  ExternalLink,
  Copy,
  Share2
} from 'lucide-react';
import { format, isToday, isYesterday, subDays, startOfDay, endOfDay, isWithinInterval } from 'date-fns';
import { BaseNotification, NotificationType, NotificationPriority, NotificationAction } from '@/types/notifications';
import { useNotifications } from '@/contexts/NotificationContext';
import { NotificationService, NotificationFilter, NotificationAnalytics } from '@/services/notificationService';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Checkbox } from '@/components/ui/checkbox';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from '@/components/ui/dropdown-menu';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from '@/components/ui/dialog';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

interface EnhancedNotificationCenterProps {
  isOpen: boolean;
  onClose: () => void;
}

const NOTIFICATION_ICONS: Record<NotificationType, React.ComponentType<{ className?: string }>> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const PRIORITY_ICONS: Record<NotificationPriority, React.ComponentType<{ className?: string }>> = {
  low: Clock,
  medium: Activity,
  high: TrendingUp,
  critical: Zap,
};

const PRIORITY_COLORS: Record<NotificationPriority, string> = {
  low: 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200',
  medium: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
  high: 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200',
  critical: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
};

const TYPE_COLORS: Record<NotificationType, string> = {
  success: 'text-green-500',
  error: 'text-red-500',
  warning: 'text-yellow-500',
  info: 'text-blue-500',
};

const CATEGORY_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  payments: Globe,
  liquidity: Database,
  snapshots: Camera,
  system: Cpu,
  security: Shield,
  network: Wifi,
  maintenance: Settings,
  updates: RefreshCw,
};

export const EnhancedNotificationCenter: React.FC<EnhancedNotificationCenterProps> = ({
  isOpen,
  onClose,
}) => {
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

  const [activeTab, setActiveTab] = useState('notifications');
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedFilters, setSelectedFilters] = useState<NotificationFilter>({});
  const [selectedNotifications, setSelectedNotifications] = useState<Set<string>>(new Set());
  const [showFilters, setShowFilters] = useState(false);
  const [groupBy, setGroupBy] = useState<'none' | 'date' | 'type' | 'priority'>('date');
  const [sortBy, setSortBy] = useState<'timestamp' | 'priority' | 'type'>('timestamp');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [viewMode, setViewMode] = useState<'list' | 'grid' | 'compact'>('list');

  // Calculate analytics
  const analytics: NotificationAnalytics = useMemo(() => {
    return NotificationService.getInstance().getAnalytics(notifications);
  }, [notifications]);

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

  return (
    <TooltipProvider>
      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.2 }}
            className="fixed inset-0 z-50 flex items-start justify-end p-4"
          >
            {/* Backdrop */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="absolute inset-0 bg-black/20 backdrop-blur-sm"
              onClick={onClose}
            />
            
            {/* Notification Panel */}
            <motion.div
              initial={{ x: 100, opacity: 0 }}
              animate={{ x: 0, opacity: 1 }}
              exit={{ x: 100, opacity: 0 }}
              transition={{ type: 'spring', damping: 25 }}
              className="relative w-full max-w-2xl h-[80vh] bg-white dark:bg-slate-900 rounded-2xl shadow-2xl border border-gray-200 dark:border-slate-700 overflow-hidden flex flex-col"
            >
              {/* Header */}
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
                      <span>{isWebSocketConnected ? (
                        <>
                          <Wifi className="h-3 w-3 text-green-500" />
                          Connected
                        </>
                      ) : (
                        <>
                          <WifiOff className="h-3 w-3 text-red-500" />
                          Disconnected
                        </>
                      )}</span>
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
                      <Button variant="ghost" size="sm" onClick={() => setShowFilters(!showFilters)}>
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

              {/* Main Content */}
              <div className="flex-1 overflow-hidden">
                <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
                  <TabsList className="grid w-full grid-cols-4 mx-6 mt-4">
                    <TabsTrigger value="notifications" className="flex items-center gap-2">
                      <Bell className="h-4 w-4" />
                      Notifications
                    </TabsTrigger>
                    <TabsTrigger value="analytics" className="flex items-center gap-2">
                      <BarChart3 className="h-4 w-4" />
                      Analytics
                    </TabsTrigger>
                    <TabsTrigger value="filters" className="flex items-center gap-2">
                      <Filter className="h-4 w-4" />
                      Filters
                    </TabsTrigger>
                    <TabsTrigger value="settings" className="flex items-center gap-2">
                      <Settings className="h-4 w-4" />
                      Settings
                    </TabsTrigger>
                  </TabsList>

                  {/* Notifications Tab */}
                  <TabsContent value="notifications" className="flex-1 overflow-hidden m-0">
                    <div className="h-full flex flex-col">
                      {/* Search and Actions Bar */}
                      <div className="p-4 border-b border-gray-200 dark:border-slate-700 space-y-3">
                        {/* Search */}
                        <div className="relative">
                          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-gray-400" />
                          <Input
                            placeholder="Search notifications..."
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                            className="pl-10"
                          />
                        </div>

                        {/* Actions Bar */}
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2">
                            <Select value={groupBy} onValueChange={(value: any) => setGroupBy(value)}>
                              <SelectTrigger className="w-32">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="none">No Group</SelectItem>
                                <SelectItem value="date">Group by Date</SelectItem>
                                <SelectItem value="type">Group by Type</SelectItem>
                                <SelectItem value="priority">Group by Priority</SelectItem>
                              </SelectContent>
                            </Select>

                            <Select value={sortBy} onValueChange={(value: any) => setSortBy(value)}>
                              <SelectTrigger className="w-32">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="timestamp">Sort by Time</SelectItem>
                                <SelectItem value="priority">Sort by Priority</SelectItem>
                                <SelectItem value="type">Sort by Type</SelectItem>
                              </SelectContent>
                            </Select>

                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc')}
                            >
                              {sortOrder === 'asc' ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                            </Button>

                            <Select value={viewMode} onValueChange={(value: any) => setViewMode(value)}>
                              <SelectTrigger className="w-24">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="list">List</SelectItem>
                                <SelectItem value="grid">Grid</SelectItem>
                                <SelectItem value="compact">Compact</SelectItem>
                              </SelectContent>
                            </Select>
                          </div>

                          <div className="flex items-center gap-2">
                            <Checkbox
                              checked={selectedNotifications.size === filteredNotifications.length && filteredNotifications.length > 0}
                              onCheckedChange={handleSelectAll}
                            />
                            <span className="text-sm text-gray-600 dark:text-gray-400">
                              {selectedNotifications.size > 0 && `${selectedNotifications.size} selected`}
                            </span>
                            
                            {selectedNotifications.size > 0 && (
                              <div className="flex items-center gap-1">
                                <Tooltip>
                                  <TooltipTrigger asChild>
                                    <Button variant="ghost" size="sm" onClick={handleBulkMarkAsRead}>
                                      <Eye className="h-4 w-4" />
                                    </Button>
                                  </TooltipTrigger>
                                  <TooltipContent>Mark Selected as Read</TooltipContent>
                                </Tooltip>
                                
                                <Tooltip>
                                  <TooltipTrigger asChild>
                                    <Button variant="ghost" size="sm" onClick={handleBulkExport}>
                                      <Download className="h-4 w-4" />
                                    </Button>
                                  </TooltipTrigger>
                                  <TooltipContent>Export Selected</TooltipContent>
                                </Tooltip>
                                
                                <Tooltip>
                                  <TooltipTrigger asChild>
                                    <Button variant="ghost" size="sm" onClick={handleBulkDelete}>
                                      <Trash2 className="h-4 w-4" />
                                    </Button>
                                  </TooltipTrigger>
                                  <TooltipContent>Delete Selected</TooltipContent>
                                </Tooltip>
                              </div>
                            )}
                          </div>
                        </div>
                      </div>

                      {/* Notifications List */}
                      <div className="flex-1 overflow-y-auto p-4">
                        {Object.entries(groupedNotifications).map(([groupName, groupNotifications]) => (
                          <div key={groupName} className="mb-6">
                            {groupName && (
                              <h3 className="text-sm font-semibold text-gray-500 dark:text-gray-400 mb-3 sticky top-0 bg-white dark:bg-slate-900 py-2">
                                {groupName} ({groupNotifications.length})
                              </h3>
                            )}
                            
                            <div className={viewMode === 'grid' ? 'grid grid-cols-1 md:grid-cols-2 gap-3' : 'space-y-2'}>
                              {groupNotifications.map(notification => (
                                <motion.div
                                  key={notification.id}
                                  initial={{ opacity: 0, y: 10 }}
                                  animate={{ opacity: 1, y: 0 }}
                                  className={`
                                    relative p-4 rounded-lg border transition-all cursor-pointer
                                    ${notification.read 
                                      ? 'bg-white dark:bg-slate-900 border-gray-200 dark:border-slate-700' 
                                      : 'bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800'
                                    }
                                    hover:shadow-md hover:border-blue-300 dark:hover:border-blue-600
                                  `}
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
                                          <NOTIFICATION_ICONS[notification.type] className={`h-4 w-4 ${TYPE_COLORS[notification.type]}`} />
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
                                              {notification.metadata.source}
                                            </span>
                                          )}
                                        </div>
                                      </div>

                                      {/* Actions */}
                                      <div className="flex items-center gap-1">
                                        <DropdownMenu>
                                          <DropdownMenuTrigger asChild>
                                            <Button variant="ghost" size="sm" onClick={(e) => e.stopPropagation()}>
                                              <MoreVertical className="h-4 w-4" />
                                            </Button>
                                          </DropdownMenuTrigger>
                                          <DropdownMenuContent align="end">
                                            <DropdownMenuItem onClick={(e) => {
                                              e.stopPropagation();
                                              markAsRead(notification.id);
                                            }}>
                                              <Eye className="h-4 w-4 mr-2" />
                                              Mark as Read
                                            </DropdownMenuItem>
                                            <DropdownMenuItem onClick={(e) => {
                                              e.stopPropagation();
                                              copyNotificationText(notification);
                                            }}>
                                              <Copy className="h-4 w-4 mr-2" />
                                              Copy
                                            </DropdownMenuItem>
                                            <DropdownMenuItem onClick={(e) => {
                                              e.stopPropagation();
                                              shareNotification(notification);
                                            }}>
                                              <Share2 className="h-4 w-4 mr-2" />
                                              Share
                                            </DropdownMenuItem>
                                            <DropdownMenuSeparator />
                                            {notification.actions?.map(action => (
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
                              ))}
                            </div>
                          </div>
                        ))}
                        
                        {filteredNotifications.length === 0 && (
                          <div className="text-center py-12">
                            <BellOff className="h-12 w-12 mx-auto mb-4 text-gray-400" />
                            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
                              No notifications found
                            </h3>
                            <p className="text-gray-600 dark:text-gray-400">
                              {searchTerm ? 'Try adjusting your search terms' : 'You\'re all caught up!'}
                            </p>
                          </div>
                        )}
                      </div>
                    </div>
                  </TabsContent>

                  {/* Analytics Tab */}
                  <TabsContent value="analytics" className="flex-1 overflow-hidden m-0 p-6">
                    <div className="h-full overflow-y-auto space-y-6">
                      {/* Summary Cards */}
                      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                        <Card>
                          <CardContent className="p-4 text-center">
                            <div className="text-2xl font-bold text-blue-600">
                              {analytics.totalNotifications}
                            </div>
                            <div className="text-sm text-gray-600">Total Notifications</div>
                          </CardContent>
                        </Card>
                        <Card>
                          <CardContent className="p-4 text-center">
                            <div className="text-2xl font-bold text-green-600">
                              {analytics.unreadCount}
                            </div>
                            <div className="text-sm text-gray-600">Unread</div>
                          </CardContent>
                        </Card>
                        <Card>
                          <CardContent className="p-4 text-center">
                            <div className="text-2xl font-bold text-orange-600">
                              {Math.round((analytics.unreadCount / analytics.totalNotifications) * 100) || 0}%
                            </div>
                            <div className="text-sm text-gray-600">Unread Rate</div>
                          </CardContent>
                        </Card>
                        <Card>
                          <CardContent className="p-4 text-center">
                            <div className="text-2xl font-bold text-purple-600">
                              {analytics.averageResponseTime || 0}m
                            </div>
                            <div className="text-sm text-gray-600">Avg Response Time</div>
                          </CardContent>
                        </Card>
                      </div>

                      {/* Distribution Charts */}
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                        <Card>
                          <CardHeader>
                            <CardTitle>Type Distribution</CardTitle>
                          </CardHeader>
                          <CardContent>
                            <div className="space-y-2">
                              {Object.entries(analytics.typeDistribution).map(([type, count]) => (
                                <div key={type} className="flex items-center justify-between">
                                  <div className="flex items-center gap-2">
                                    <NOTIFICATION_ICONS[type as NotificationType] className={`h-4 w-4 ${TYPE_COLORS[type as NotificationType]}`} />
                                    <span className="capitalize">{type}</span>
                                  </div>
                                  <Badge variant="secondary">{count}</Badge>
                                </div>
                              ))}
                            </div>
                          </CardContent>
                        </Card>

                        <Card>
                          <CardHeader>
                            <CardTitle>Priority Distribution</CardTitle>
                          </CardHeader>
                          <CardContent>
                            <div className="space-y-2">
                              {Object.entries(analytics.priorityDistribution).map(([priority, count]) => (
                                <div key={priority} className="flex items-center justify-between">
                                  <div className="flex items-center gap-2">
                                    <PRIORITY_ICONS[priority as NotificationPriority] className="h-4 w-4" />
                                    <span className="capitalize">{priority}</span>
                                  </div>
                                  <Badge className={PRIORITY_COLORS[priority as NotificationPriority]}>
                                    {count}
                                  </Badge>
                                </div>
                              ))}
                            </div>
                          </CardContent>
                        </Card>
                      </div>
                    </div>
                  </TabsContent>

                  {/* Filters Tab */}
                  <TabsContent value="filters" className="flex-1 overflow-hidden m-0 p-6">
                    <div className="max-w-2xl space-y-6">
                      <Card>
                        <CardHeader>
                          <CardTitle>Filter Options</CardTitle>
                          <CardDescription>
                            Customize which notifications you want to see
                          </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-6">
                          {/* Date Range */}
                          <div>
                            <Label className="text-sm font-medium">Date Range</Label>
                            <div className="grid grid-cols-2 gap-4 mt-2">
                              <Input
                                type="date"
                                value={selectedFilters.dateRange?.start ? format(selectedFilters.dateRange.start, 'yyyy-MM-dd') : ''}
                                onChange={(e) => {
                                  const start = e.target.value ? new Date(e.target.value) : undefined;
                                  setSelectedFilters(prev => ({
                                    ...prev,
                                    dateRange: prev.dateRange ? { ...prev.dateRange, start } : start ? { start, end: new Date() } : undefined
                                  }));
                                }}
                              />
                              <Input
                                type="date"
                                value={selectedFilters.dateRange?.end ? format(selectedFilters.dateRange.end, 'yyyy-MM-dd') : ''}
                                onChange={(e) => {
                                  const end = e.target.value ? new Date(e.target.value) : undefined;
                                  setSelectedFilters(prev => ({
                                    ...prev,
                                    dateRange: prev.dateRange ? { ...prev.dateRange, end } : end ? { start: new Date(), end } : undefined
                                  }));
                                }}
                              />
                            </div>
                          </div>

                          {/* Types */}
                          <div>
                            <Label className="text-sm font-medium">Types</Label>
                            <div className="grid grid-cols-2 gap-2 mt-2">
                              {(['success', 'error', 'warning', 'info'] as NotificationType[]).map(type => (
                                <div key={type} className="flex items-center space-x-2">
                                  <Checkbox
                                    id={`type-${type}`}
                                    checked={selectedFilters.types?.includes(type) || false}
                                    onCheckedChange={(checked) => {
                                      setSelectedFilters(prev => ({
                                        ...prev,
                                        types: checked 
                                          ? [...(prev.types || []), type]
                                          : (prev.types || []).filter(t => t !== type)
                                      }));
                                    }}
                                  />
                                  <Label htmlFor={`type-${type}`} className="flex items-center gap-2">
                                    <NOTIFICATION_ICONS[type] className={`h-4 w-4 ${TYPE_COLORS[type]}`} />
                                    <span className="capitalize">{type}</span>
                                  </Label>
                                </div>
                              ))}
                            </div>
                          </div>

                          {/* Priorities */}
                          <div>
                            <Label className="text-sm font-medium">Priorities</Label>
                            <div className="grid grid-cols-2 gap-2 mt-2">
                              {(['low', 'medium', 'high', 'critical'] as NotificationPriority[]).map(priority => (
                                <div key={priority} className="flex items-center space-x-2">
                                  <Checkbox
                                    id={`priority-${priority}`}
                                    checked={selectedFilters.priorities?.includes(priority) || false}
                                    onCheckedChange={(checked) => {
                                      setSelectedFilters(prev => ({
                                        ...prev,
                                        priorities: checked 
                                          ? [...(prev.priorities || []), priority]
                                          : (prev.priorities || []).filter(p => p !== priority)
                                      }));
                                    }}
                                  />
                                  <Label htmlFor={`priority-${priority}`} className="flex items-center gap-2">
                                    <Badge className={PRIORITY_COLORS[priority]}>
                                      {priority}
                                    </Badge>
                                  </Label>
                                </div>
                              ))}
                            </div>
                          </div>

                          {/* Categories */}
                          <div>
                            <Label className="text-sm font-medium">Categories</Label>
                            <div className="grid grid-cols-2 gap-2 mt-2">
                              {['payments', 'liquidity', 'snapshots', 'system', 'security', 'network', 'maintenance', 'updates'].map(category => (
                                <div key={category} className="flex items-center space-x-2">
                                  <Checkbox
                                    id={`category-${category}`}
                                    checked={selectedFilters.categories?.includes(category) || false}
                                    onCheckedChange={(checked) => {
                                      setSelectedFilters(prev => ({
                                        ...prev,
                                        categories: checked 
                                          ? [...(prev.categories || []), category]
                                          : (prev.categories || []).filter(c => c !== category)
                                      }));
                                    }}
                                  />
                                  <Label htmlFor={`category-${category}`} className="flex items-center gap-2">
                                    {CATEGORY_ICONS[category] && (
                                      React.createElement(CATEGORY_ICONS[category], { className: 'h-4 w-4' })
                                    )}
                                    <span className="capitalize">{category}</span>
                                  </Label>
                                </div>
                              ))}
                            </div>
                          </div>

                          {/* Read Status */}
                          <div>
                            <Label className="text-sm font-medium">Read Status</Label>
                            <div className="flex gap-4 mt-2">
                              {['all', 'read', 'unread'].map(status => (
                                <div key={status} className="flex items-center space-x-2">
                                  <input
                                    type="radio"
                                    id={`status-${status}`}
                                    name="read-status"
                                    checked={selectedFilters.readStatus === status || (!selectedFilters.readStatus && status === 'all')}
                                    onChange={() => {
                                      setSelectedFilters(prev => ({
                                        ...prev,
                                        readStatus: status as any
                                      }));
                                    }}
                                  />
                                  <Label htmlFor={`status-${status}`} className="capitalize">
                                    {status}
                                  </Label>
                                </div>
                              ))}
                            </div>
                          </div>

                          {/* Clear Filters */}
                          <div className="flex gap-2">
                            <Button 
                              variant="outline" 
                              onClick={() => setSelectedFilters({})}
                            >
                              Clear All Filters
                            </Button>
                            <Button onClick={() => setShowFilters(false)}>
                              Apply Filters
                            </Button>
                          </div>
                        </CardContent>
                      </Card>
                    </div>
                  </TabsContent>

                  {/* Settings Tab */}
                  <TabsContent value="settings" className="flex-1 overflow-hidden m-0 p-6">
                    <div className="max-w-2xl space-y-6">
                      <Card>
                        <CardHeader>
                          <CardTitle>Notification Preferences</CardTitle>
                          <CardDescription>
                            Configure how you receive notifications
                          </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-6">
                          {/* Enable Notifications */}
                          <div className="flex items-center justify-between">
                            <div>
                              <Label className="text-sm font-medium">Enable Notifications</Label>
                              <p className="text-sm text-gray-600">
                                Turn notifications on or off globally
                              </p>
                            </div>
                            <Switch
                              checked={preferences.enabled}
                              onCheckedChange={(enabled) => updatePreferences({ enabled })}
                            />
                          </div>

                          {/* Desktop Notifications */}
                          <div className="flex items-center justify-between">
                            <div>
                              <Label className="text-sm font-medium">Desktop Notifications</Label>
                              <p className="text-sm text-gray-600">
                                Show notifications on your desktop
                              </p>
                            </div>
                            <Switch
                              checked={preferences.showOnDesktop}
                              onCheckedChange={(showOnDesktop) => updatePreferences({ showOnDesktop })}
                            />
                          </div>

                          {/* Sound */}
                          <div className="space-y-4">
                            <div className="flex items-center justify-between">
                              <div>
                                <Label className="text-sm font-medium">Sound</Label>
                                <p className="text-sm text-gray-600">
                                  Play sound for notifications
                                </p>
                              </div>
                              <Switch
                                checked={preferences.sound.enabled}
                                onCheckedChange={(enabled) => updatePreferences({ 
                                  sound: { ...preferences.sound, enabled }
                                })}
                              />
                            </div>
                            
                            {preferences.sound.enabled && (
                              <div className="space-y-4 pl-4 border-l-2 border-gray-200 dark:border-slate-700">
                                <div>
                                  <Label className="text-sm font-medium">Volume</Label>
                                  <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.1"
                                    value={preferences.sound.volume}
                                    onChange={(e) => updatePreferences({
                                      sound: { ...preferences.sound, volume: parseFloat(e.target.value) }
                                    })}
                                    className="w-full mt-2"
                                  />
                                </div>
                                
                                <div>
                                  <Label className="text-sm font-medium">Sound Type</Label>
                                  <Select 
                                    value={preferences.sound.soundType}
                                    onValueChange={(soundType: any) => updatePreferences({
                                      sound: { ...preferences.sound, soundType }
                                    })}
                                  >
                                    <SelectTrigger className="w-full mt-2">
                                      <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                      <SelectItem value="default">Default</SelectItem>
                                      <SelectItem value="subtle">Subtle</SelectItem>
                                      <SelectItem value="alert">Alert</SelectItem>
                                      <SelectItem value="critical">Critical</SelectItem>
                                    </SelectContent>
                                  </Select>
                                </div>
                              </div>
                            )}
                          </div>

                          {/* Auto-hide */}
                          <div className="space-y-4">
                            <div className="flex items-center justify-between">
                              <div>
                                <Label className="text-sm font-medium">Auto-hide</Label>
                                <p className="text-sm text-gray-600">
                                  Automatically hide notifications after a delay
                                </p>
                              </div>
                              <Switch
                                checked={preferences.autoHide}
                                onCheckedChange={(autoHide) => updatePreferences({ autoHide })}
                              />
                            </div>
                            
                            {preferences.autoHide && (
                              <div className="pl-4 border-l-2 border-gray-200 dark:border-slate-700">
                                <Label className="text-sm font-medium">Delay (seconds)</Label>
                                <Input
                                  type="number"
                                  min="1"
                                  max="30"
                                  value={preferences.autoHideDelay / 1000}
                                  onChange={(e) => updatePreferences({
                                    autoHideDelay: parseInt(e.target.value) * 1000
                                  })}
                                  className="w-full mt-2"
                                />
                              </div>
                            )}
                          </div>

                          {/* Categories */}
                          <div>
                            <Label className="text-sm font-medium mb-4 block">Categories</Label>
                            <div className="space-y-3">
                              {Object.entries(preferences.categories).map(([category, enabled]) => (
                                <div key={category} className="flex items-center justify-between">
                                  <div className="flex items-center gap-2">
                                    {CATEGORY_ICONS[category] && (
                                      React.createElement(CATEGORY_ICONS[category], { className: 'h-4 w-4' })
                                    )}
                                    <span className="capitalize">{category}</span>
                                  </div>
                                  <Switch
                                    checked={enabled}
                                    onCheckedChange={(categoryEnabled) => updatePreferences({
                                      categories: {
                                        ...preferences.categories,
                                        [category]: categoryEnabled
                                      }
                                    })}
                                  />
                                </div>
                              ))}
                            </div>
                          </div>
                        </CardContent>
                      </Card>
                    </div>
                  </TabsContent>
                </Tabs>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </TooltipProvider>
  );
};
