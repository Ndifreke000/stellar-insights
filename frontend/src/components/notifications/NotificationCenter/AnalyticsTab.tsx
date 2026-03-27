import { NotificationAnalytics } from "@/services/notificationService";
import { AlertCircle, Bell, CheckCircle, Clock } from "lucide-react";
import { ICON_COLORS, NOTIFICATION_ICONS, PRIORITY_BADGES } from "./helpers";
import { NotificationPriority, NotificationType } from "@/types/notifications";

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

export default AnalyticsTab