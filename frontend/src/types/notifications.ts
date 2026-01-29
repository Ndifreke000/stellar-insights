export type NotificationType = 'success' | 'error' | 'warning' | 'info';

export type NotificationPriority = 'low' | 'medium' | 'high' | 'critical';

export interface NotificationSound {
  enabled: boolean;
  volume: number;
  soundType: 'default' | 'subtle' | 'alert' | 'critical';
}

export interface NotificationPreferences {
  enabled: boolean;
  sound: NotificationSound;
  showOnDesktop: boolean;
  autoHide: boolean;
  autoHideDelay: number;
  categories: {
    payments: boolean;
    liquidity: boolean;
    snapshots: boolean;
    system: boolean;
  };
}

export interface BaseNotification {
  id: string;
  type: NotificationType;
  priority: NotificationPriority;
  title: string;
  message: string;
  category: keyof NotificationPreferences['categories'];
  timestamp: Date;
  read: boolean;
  persistent?: boolean;
  actions?: NotificationAction[];
  metadata?: Record<string, any>;
}

export interface NotificationAction {
  id: string;
  label: string;
  variant?: 'primary' | 'secondary' | 'destructive';
  onClick: () => void | Promise<void>;
}

export interface ToastNotification extends BaseNotification {
  duration?: number;
  dismissible?: boolean;
  position?: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left' | 'top-center' | 'bottom-center';
}

export interface WebSocketNotificationPayload {
  type: 'payment_failed' | 'low_liquidity' | 'new_snapshot' | 'system_alert';
  data: {
    title: string;
    message: string;
    priority: NotificationPriority;
    metadata?: Record<string, any>;
  };
}

export interface NotificationContextType {
  notifications: BaseNotification[];
  preferences: NotificationPreferences;
  showToast: (notification: Omit<ToastNotification, 'id' | 'timestamp' | 'read'>) => string;
  dismissToast: (id: string) => void;
  markAsRead: (id: string) => void;
  markAllAsRead: () => void;
  clearNotification: (id: string) => void;
  clearAllNotifications: () => void;
  updatePreferences: (preferences: Partial<NotificationPreferences>) => void;
  unreadCount: number;
  isWebSocketConnected: boolean;
  webSocketReconnectCount: number;
}