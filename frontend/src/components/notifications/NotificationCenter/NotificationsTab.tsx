import { NotificationPriority, NotificationType } from "@/types/notifications";
import { motion, AnimatePresence } from "framer-motion";
import { BellOff, Calendar, ChevronDown, ChevronUp, Download, Filter, Search, Trash2 } from "lucide-react";
import { formatTime, ICON_COLORS, NOTIFICATION_ICONS, NotificationCenterProps, PRIORITY_BADGES } from "./helpers";

const NotificationsTab = ({
  searchTerm,
  setSearchTerm,
  updateFilter,
  setShowFilters,
  showFilters,
  filter,
  handleSelectAll,
  handleBatchMarkAsRead,
  handleBatchDelete,
  handleExport,
  markAllAsRead,
  clearAllNotifications,
  selectedNotifications,
  groupedNotifications,
  unreadCount,
  handleNotificationClick,
  handleSelectNotification,
  filteredNotifications,
  clearNotification,
}:NotificationCenterProps) => {
  return (
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
              className={`px-4 py-2 rounded-lg transition-colors flex items-center space-x-2 ${showFilters
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
                              onChange={(e: any) => handleSelectNotification(notification.id, e)}
                              onClick={(e) => e.stopPropagation()}
                              className="mt-1 rounded"
                            />
                            <Icon className={`h-5 w-5 mt-0.5 ${ICON_COLORS[notification.type]}`} />
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center justify-between">
                                <h4 className={`text-sm font-medium ${notification.read
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
                              <p className={`text-sm mt-1 ${notification.read
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
  );
}

export default NotificationsTab;