import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  NOTIFICATION_ICONS,
  PRIORITY_COLORS,
  PRIORITY_ICONS,
  TYPE_COLORS,
} from './util';
import { Badge } from '@/components/ui/badge';
import { NotificationPriority, NotificationType } from '@/types/notifications';
import {
  NotificationAnalytics,
  NotificationService,
} from '@/services/notificationService';
import { useMemo } from 'react';
import { useNotifications } from '@/contexts/NotificationContext';

const AnalyticsView = () => {
  const { notifications } = useNotifications();

  const analytics: NotificationAnalytics = useMemo(() => {
    return NotificationService.getInstance().getAnalytics(notifications);
  }, [notifications]);

  return (
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
              {Math.round(
                (analytics.unreadCount / analytics.totalNotifications) * 100,
              ) || 0}
              %
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
              {Object.entries(analytics.typeDistribution).map(
                ([type, count]) => {
                  const Icon = NOTIFICATION_ICONS[type as NotificationType];
                  return (
                    <div
                      key={type}
                      className="flex items-center justify-between"
                    >
                      <div className="flex items-center gap-2">
                        <Icon
                          className={`h-4 w-4 ${TYPE_COLORS[type as NotificationType]}`}
                        />
                        <span className="capitalize">{type}</span>
                      </div>
                      <Badge variant="secondary">{count}</Badge>
                    </div>
                  );
                },
              )}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Priority Distribution</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {Object.entries(analytics.priorityDistribution).map(
                ([priority, count]) => {
                  const Icon = PRIORITY_ICONS[priority as NotificationPriority];
                  return (
                    <div
                      key={priority}
                      className="flex items-center justify-between"
                    >
                      <div className="flex items-center gap-2">
                        <Icon className="h-4 w-4" />
                        <span className="capitalize">{priority}</span>
                      </div>
                      <Badge
                        className={
                          PRIORITY_COLORS[priority as NotificationPriority]
                        }
                      >
                        {count}
                      </Badge>
                    </div>
                  );
                },
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
};

export default AnalyticsView;
