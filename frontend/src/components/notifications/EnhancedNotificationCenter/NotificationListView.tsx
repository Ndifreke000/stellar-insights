import { motion } from 'framer-motion';
import {
  Search,
  ChevronUp,
  ChevronDown,
  BellOff,
  Eye,
  Download,
  Trash2,
} from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Checkbox } from '@/components/ui/checkbox';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';


import { NotificationItem } from './NotificationItem';
import { IUseEnhancedNotificationCenter } from './useEnhancedNotificationCenter';

export const NotificationListView = (props: Partial<IUseEnhancedNotificationCenter>) => {
  const {
    markAsRead,
    clearNotification,
    filteredNotifications,
    groupedNotifications,
    selectedNotifications,
    setSelectedNotifications,
    handleSelectAll,
    handleBulkMarkAsRead,
    handleBulkDelete,
    handleBulkExport,
    handleNotificationAction,
    copyNotificationText,
    shareNotification,
    searchTerm,
    setSearchTerm,
    groupBy,
    setGroupBy,
    sortBy,
    setSortBy,
    sortOrder,
    setSortOrder,
    viewMode,
    setViewMode,
  } = props;
  return (
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
            <Select
              value={groupBy}
              onValueChange={(value: any) => setGroupBy(value)}
            >
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

            <Select
              value={sortBy}
              onValueChange={(value: any) => setSortBy(value)}
            >
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
              {sortOrder === 'asc' ? (
                <ChevronUp className="h-4 w-4" />
              ) : (
                <ChevronDown className="h-4 w-4" />
              )}
            </Button>

            <Select
              value={viewMode}
              onValueChange={(value: any) => setViewMode(value)}
            >
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
              checked={
                selectedNotifications.size === filteredNotifications.length &&
                filteredNotifications.length > 0
              }
              onCheckedChange={handleSelectAll}
            />
            <span className="text-sm text-gray-600 dark:text-gray-400">
              {selectedNotifications.size > 0 &&
                `${selectedNotifications.size} selected`}
            </span>

            {selectedNotifications.size > 0 && (
              <div className="flex items-center gap-1">
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={handleBulkMarkAsRead}
                    >
                      <Eye className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Mark Selected as Read</TooltipContent>
                </Tooltip>

                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={handleBulkExport}
                    >
                      <Download className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Export Selected</TooltipContent>
                </Tooltip>

                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={handleBulkDelete}
                    >
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
        {Object.entries(groupedNotifications).map(
          ([groupName, groupNotifications]) => (
            <div key={groupName} className="mb-6">
              {groupName && (
                <h3 className="text-sm font-semibold text-gray-500 dark:text-gray-400 mb-3 sticky top-0 bg-white dark:bg-slate-900 py-2">
                  {groupName} ({groupNotifications.length})
                </h3>
              )}

              <div
                className={
                  viewMode === 'grid'
                    ? 'grid grid-cols-1 md:grid-cols-2 gap-3'
                    : 'space-y-2'
                }
              >
                {groupNotifications.map((notification, index) => (
                  <NotificationItem
                    key={notification.id}
                    notification={notification}
                    selectedNotifications={selectedNotifications}
                    setSelectedNotifications={setSelectedNotifications}
                    markAsRead={markAsRead}
                    clearNotification={clearNotification}
                    copyNotificationText={copyNotificationText}
                    shareNotification={shareNotification}
                    handleNotificationAction={handleNotificationAction}
                    viewMode={viewMode}
                  />
                ))}
              </div>
            </div>
          ),
        )}

        {filteredNotifications.length === 0 && (
          <div className="text-center py-12">
            <BellOff className="h-12 w-12 mx-auto mb-4 text-gray-400" />
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
              No notifications found
            </h3>
            <p className="text-gray-600 dark:text-gray-400">
              {searchTerm
                ? 'Try adjusting your search terms'
                : "You're all caught up!"}
            </p>
          </div>
        )}
      </div>
    </div>
  );
};
