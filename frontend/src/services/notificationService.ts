'use client';

import { BaseNotification, NotificationType, NotificationPriority, NotificationAction } from '@/types/notifications';

export interface NotificationTemplate {
  id: string;
  name: string;
  type: NotificationType;
  priority: NotificationPriority;
  category: keyof NotificationPreferences['categories'];
  titleTemplate: string;
  messageTemplate: string;
  defaultActions?: NotificationAction[];
  variables?: string[];
}

export interface NotificationFilter {
  dateRange?: {
    start: Date;
    end: Date;
  };
  types?: NotificationType[];
  priorities?: NotificationPriority[];
  categories?: string[];
  readStatus?: 'all' | 'read' | 'unread';
  searchTerm?: string;
}

export interface NotificationAnalytics {
  totalNotifications: number;
  unreadCount: number;
  typeDistribution: Record<NotificationType, number>;
  priorityDistribution: Record<NotificationPriority, number>;
  categoryDistribution: Record<string, number>;
  dailyStats: Array<{
    date: string;
    count: number;
    unread: number;
  }>;
  averageResponseTime?: number; // in minutes
}

export class NotificationService {
  private static instance: NotificationService;
  private templates: Map<string, NotificationTemplate> = new Map();

  static getInstance(): NotificationService {
    if (!NotificationService.instance) {
      NotificationService.instance = new NotificationService();
    }
    return NotificationService.instance;
  }

  // Template Management
  registerTemplate(template: NotificationTemplate): void {
    this.templates.set(template.id, template);
  }

  getTemplate(id: string): NotificationTemplate | undefined {
    return this.templates.get(id);
  }

  getAllTemplates(): NotificationTemplate[] {
    return Array.from(this.templates.values());
  }

  // Create notification from template
  createFromTemplate(
    templateId: string,
    variables: Record<string, string | number>,
    customActions?: NotificationAction[]
  ): BaseNotification | null {
    const template = this.getTemplate(templateId);
    if (!template) return null;

    const title = this.interpolateTemplate(template.titleTemplate, variables);
    const message = this.interpolateTemplate(template.messageTemplate, variables);

    return {
      id: `notification-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      type: template.type,
      priority: template.priority,
      category: template.category,
      title,
      message,
      timestamp: new Date(),
      read: false,
      actions: customActions || template.defaultActions,
      metadata: { templateId, variables },
    };
  }

  private interpolateTemplate(template: string, variables: Record<string, string | number>): string {
    return template.replace(/\{\{(\w+)\}\}/g, (match, key) => {
      return variables[key]?.toString() || match;
    });
  }

  // Filtering and Search
  filterNotifications(notifications: BaseNotification[], filter: NotificationFilter): BaseNotification[] {
    return notifications.filter(notification => {
      // Date range filter
      if (filter.dateRange) {
        const notifDate = new Date(notification.timestamp);
        if (notifDate < filter.dateRange.start || notifDate > filter.dateRange.end) {
          return false;
        }
      }

      // Type filter
      if (filter.types && filter.types.length > 0) {
        if (!filter.types.includes(notification.type)) {
          return false;
        }
      }

      // Priority filter
      if (filter.priorities && filter.priorities.length > 0) {
        if (!filter.priorities.includes(notification.priority)) {
          return false;
        }
      }

      // Category filter
      if (filter.categories && filter.categories.length > 0) {
        if (!filter.categories.includes(notification.category)) {
          return false;
        }
      }

      // Read status filter
      if (filter.readStatus === 'read' && !notification.read) {
        return false;
      }
      if (filter.readStatus === 'unread' && notification.read) {
        return false;
      }

      // Search term filter
      if (filter.searchTerm) {
        const searchLower = filter.searchTerm.toLowerCase();
        const titleMatch = notification.title.toLowerCase().includes(searchLower);
        const messageMatch = notification.message.toLowerCase().includes(searchLower);
        if (!titleMatch && !messageMatch) {
          return false;
        }
      }

      return true;
    });
  }

  // Analytics
  generateAnalytics(notifications: BaseNotification[]): NotificationAnalytics {
    const typeDistribution: Record<NotificationType, number> = {
      success: 0,
      error: 0,
      warning: 0,
      info: 0,
    };

    const priorityDistribution: Record<NotificationPriority, number> = {
      low: 0,
      medium: 0,
      high: 0,
      critical: 0,
    };

    const categoryDistribution: Record<string, number> = {};

    // Daily stats for last 30 days
    const dailyStats: Array<{ date: string; count: number; unread: number }> = [];
    const today = new Date();
    
    for (let i = 29; i >= 0; i--) {
      const date = new Date(today);
      date.setDate(date.getDate() - i);
      const dateStr = date.toISOString().split('T')[0];
      dailyStats.push({ date: dateStr, count: 0, unread: 0 });
    }

    let totalResponseTime = 0;
    let responseTimeCount = 0;

    notifications.forEach(notification => {
      // Type distribution
      typeDistribution[notification.type]++;

      // Priority distribution
      priorityDistribution[notification.priority]++;

      // Category distribution
      categoryDistribution[notification.category] = (categoryDistribution[notification.category] || 0) + 1;

      // Daily stats
      const notifDate = new Date(notification.timestamp);
      const dateStr = notifDate.toISOString().split('T')[0];
      const dayStat = dailyStats.find(stat => stat.date === dateStr);
      if (dayStat) {
        dayStat.count++;
        if (!notification.read) {
          dayStat.unread++;
        }
      }

      // Response time calculation (if notification has read timestamp)
      if (notification.read && notification.metadata?.readAt) {
        const readAt = new Date(notification.metadata.readAt as string);
        const responseTime = (readAt.getTime() - notifDate.getTime()) / (1000 * 60); // minutes
        totalResponseTime += responseTime;
        responseTimeCount++;
      }
    });

    return {
      totalNotifications: notifications.length,
      unreadCount: notifications.filter(n => !n.read).length,
      typeDistribution,
      priorityDistribution,
      categoryDistribution,
      dailyStats,
      averageResponseTime: responseTimeCount > 0 ? totalResponseTime / responseTimeCount : undefined,
    };
  }

  // Export functionality
  exportNotifications(notifications: BaseNotification[], format: 'json' | 'csv'): string {
    if (format === 'json') {
      return JSON.stringify(notifications, null, 2);
    }

    // CSV format
    const headers = ['ID', 'Type', 'Priority', 'Category', 'Title', 'Message', 'Timestamp', 'Read'];
    const rows = notifications.map(notification => [
      notification.id,
      notification.type,
      notification.priority,
      notification.category,
      notification.title,
      notification.message,
      notification.timestamp.toISOString(),
      notification.read.toString(),
    ]);

    return [headers, ...rows]
      .map(row => row.map(cell => `"${cell}"`).join(','))
      .join('\n');
  }

  // Smart grouping
  groupRelatedNotifications(notifications: BaseNotification[]): Map<string, BaseNotification[]> {
    const groups = new Map<string, BaseNotification[]>();

    notifications.forEach(notification => {
      // Group by template ID if available
      if (notification.metadata?.templateId) {
        const templateId = notification.metadata.templateId as string;
        if (!groups.has(templateId)) {
          groups.set(templateId, []);
        }
        groups.get(templateId)!.push(notification);
        return;
      }

      // Group by category + type combination
      const groupKey = `${notification.category}-${notification.type}`;
      if (!groups.has(groupKey)) {
        groups.set(groupKey, []);
      }
      groups.get(groupKey)!.push(notification);
    });

    return groups;
  }

  // Batch operations
  batchMarkAsRead(notifications: BaseNotification[], notificationIds: string[]): BaseNotification[] {
    const idSet = new Set(notificationIds);
    return notifications.map(notification =>
      idSet.has(notification.id) ? { ...notification, read: true, metadata: { ...notification.metadata, readAt: new Date().toISOString() } } : notification
    );
  }

  batchDelete(notifications: BaseNotification[], notificationIds: string[]): BaseNotification[] {
    const idSet = new Set(notificationIds);
    return notifications.filter(notification => !idSet.has(notification.id));
  }
}

// Initialize default templates
const notificationService = NotificationService.getInstance();

// Payment success template
notificationService.registerTemplate({
  id: 'payment-success',
  name: 'Payment Success',
  type: 'success',
  priority: 'medium',
  category: 'payments',
  titleTemplate: 'Payment Completed Successfully',
  messageTemplate: 'Payment of {{amount}} XLM to {{recipient}} was completed in {{duration}}s.',
  variables: ['amount', 'recipient', 'duration'],
});

// Payment failed template
notificationService.registerTemplate({
  id: 'payment-failed',
  name: 'Payment Failed',
  type: 'error',
  priority: 'high',
  category: 'payments',
  titleTemplate: 'Payment Failed',
  messageTemplate: 'Payment of {{amount}} XLM to {{recipient}} failed: {{reason}}',
  variables: ['amount', 'recipient', 'reason'],
});

// Low liquidity warning
notificationService.registerTemplate({
  id: 'low-liquidity',
  name: 'Low Liquidity Warning',
  type: 'warning',
  priority: 'high',
  category: 'liquidity',
  titleTemplate: 'Low Liquidity Alert',
  messageTemplate: 'Liquidity pool {{pool}} has fallen below {{threshold}}% capacity.',
  variables: ['pool', 'threshold'],
});

// System alert template
notificationService.registerTemplate({
  id: 'system-alert',
  name: 'System Alert',
  type: 'warning',
  priority: 'critical',
  category: 'system',
  titleTemplate: 'System Alert: {{alertType}}',
  messageTemplate: '{{description}} - Action required: {{action}}',
  variables: ['alertType', 'description', 'action'],
});

export default notificationService;
