'use client';

import React, { createContext, useContext, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { 
  BaseNotification, 
  NotificationContextType, 
  NotificationPreferences, 
  ToastNotification,
  WebSocketNotificationPayload,
  NotificationPriority
} from '@/types/notifications';
import { useLocalStorage } from '@/hooks/useLocalStorage';
import { useNotificationSound } from '@/hooks/useNotificationSound';
import { useWebSocket } from '@/hooks/useWebSocket';

const DEFAULT_PREFERENCES: NotificationPreferences = {
  enabled: true,
  sound: {
    enabled: true,
    volume: 0.5,
    soundType: 'default',
  },
  showOnDesktop: true,
  autoHide: true,
  autoHideDelay: 5000,
  categories: {
    payments: true,
    liquidity: true,
    snapshots: true,
    system: true,
  },
};

const NotificationContext = createContext<NotificationContextType | undefined>(undefined);

interface NotificationProviderProps {
  children: React.ReactNode;
  websocketUrl?: string;
}

export const NotificationProvider: React.FC<NotificationProviderProps> = ({ 
  children, 
  websocketUrl = process.env.NEXT_PUBLIC_WS_URL || '' // Disable WebSocket by default
}) => {
  const [notifications, setNotifications] = useLocalStorage<BaseNotification[]>('stellar-notifications', []);
  const [preferences, setPreferences] = useLocalStorage<NotificationPreferences>('stellar-notification-preferences', DEFAULT_PREFERENCES);
  const { playSound } = useNotificationSound();
  const [isClient, setIsClient] = useState(false);

  // Only initialize client-side features after hydration
  useEffect(() => {
    setIsClient(true);
  }, []);

  // WebSocket connection for real-time notifications
  const handleWebSocketMessage = useCallback((payload: WebSocketNotificationPayload) => {
    if (!isClient) return; // Don't handle messages during SSR
    
    const categoryMap = {
      'payment_failed': 'payments' as const,
      'low_liquidity': 'liquidity' as const,
      'new_snapshot': 'snapshots' as const,
      'system_alert': 'system' as const,
    };

    const typeMap = {
      'payment_failed': 'error' as const,
      'low_liquidity': 'warning' as const,
      'new_snapshot': 'info' as const,
      'system_alert': 'warning' as const,
    };

    const category = categoryMap[payload.type];
    const type = typeMap[payload.type];

    // Check if this category is enabled
    if (!preferences.enabled || !preferences.categories[category]) {
      return;
    }

    showToast({
      type,
      priority: payload.data.priority,
      title: payload.data.title,
      message: payload.data.message,
      category,
      metadata: payload.data.metadata,
    });
  }, [preferences, isClient]);

  const { isConnected, reconnectCount } = useWebSocket({
    url: websocketUrl,
    onMessage: handleWebSocketMessage,
    onConnect: () => {
      if (isClient) console.log('WebSocket connected for notifications');
    },
    onDisconnect: () => {
      if (isClient) console.log('WebSocket disconnected');
    },
    onError: (error) => {
      if (isClient) console.error('WebSocket error:', error);
    },
  });

  const generateId = useCallback(() => {
    return `notification-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }, []);

  const showToast = useCallback((
    notification: Omit<ToastNotification, 'id' | 'timestamp' | 'read'>
  ): string => {
    if (!isClient) return ''; // Don't show toasts during SSR
    
    const id = generateId();
    const newNotification: BaseNotification = {
      ...notification,
      id,
      timestamp: new Date(),
      read: false,
    };

    setNotifications(prev => [newNotification, ...prev]);

    // Play sound if enabled
    if (preferences.sound.enabled) {
      playSound(preferences.sound, notification.type, notification.priority);
    }

    // Show desktop notification if enabled and permission granted
    if (preferences.showOnDesktop && typeof window !== 'undefined' && 'Notification' in window && Notification.permission === 'granted') {
      try {
        const desktopNotification = new Notification(notification.title, {
          body: notification.message,
          icon: '/icon.svg',
          tag: id,
          requireInteraction: notification.priority === 'critical',
        });

        desktopNotification.onclick = () => {
          window.focus();
          desktopNotification.close();
        };

        // Auto-close desktop notification
        if (notification.priority !== 'critical') {
          setTimeout(() => {
            desktopNotification.close();
          }, preferences.autoHideDelay);
        }
      } catch (error) {
        console.warn('Failed to show desktop notification:', error);
      }
    }

    return id;
  }, [generateId, preferences, setNotifications, playSound, isClient]);

  const dismissToast = useCallback((id: string) => {
    setNotifications(prev => prev.filter(n => n.id !== id));
  }, [setNotifications]);

  const markAsRead = useCallback((id: string) => {
    setNotifications(prev => 
      prev.map(n => n.id === id ? { ...n, read: true } : n)
    );
  }, [setNotifications]);

  const markAllAsRead = useCallback(() => {
    setNotifications(prev => prev.map(n => ({ ...n, read: true })));
  }, [setNotifications]);

  const clearNotification = useCallback((id: string) => {
    setNotifications(prev => prev.filter(n => n.id !== id));
  }, [setNotifications]);

  const clearAllNotifications = useCallback(() => {
    setNotifications([]);
  }, [setNotifications]);

  const updatePreferences = useCallback((newPreferences: Partial<NotificationPreferences>) => {
    setPreferences(prev => ({ ...prev, ...newPreferences }));
  }, [setPreferences]);

  // Request notification permission on mount (client-side only)
  useEffect(() => {
    if (!isClient) return;
    
    if (preferences.showOnDesktop && 'Notification' in window && Notification.permission === 'default') {
      Notification.requestPermission().then(permission => {
        if (permission === 'denied') {
          updatePreferences({ showOnDesktop: false });
        }
      });
    }
  }, [preferences.showOnDesktop, updatePreferences, isClient]);

  // Clean up old notifications (keep last 100) - use a ref to avoid infinite loops
  const cleanupTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  
  useEffect(() => {
    if (!isClient) return;
    
    if (notifications.length > 100) {
      // Debounce the cleanup to avoid excessive operations
      if (cleanupTimeoutRef.current) {
        clearTimeout(cleanupTimeoutRef.current);
      }
      
      cleanupTimeoutRef.current = setTimeout(() => {
        setNotifications(prev => {
          if (prev.length > 100) {
            return [...prev]
              .sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
              .slice(0, 100);
          }
          return prev;
        });
      }, 1000);
    }
    
    return () => {
      if (cleanupTimeoutRef.current) {
        clearTimeout(cleanupTimeoutRef.current);
      }
    };
  }, [notifications.length, isClient]);

  // Show connection status notifications only if WebSocket URL is provided (client-side only)
  useEffect(() => {
    if (!isClient) return;
    
    if (websocketUrl && reconnectCount > 0) {
      showToast({
        type: 'warning',
        priority: 'medium',
        title: 'Connection Issues',
        message: `Attempting to reconnect... (${reconnectCount}/5)`,
        category: 'system',
        duration: 3000,
      });
    }
  }, [reconnectCount, showToast, websocketUrl, isClient]);

  const unreadCount = useMemo(() => {
    return notifications.filter(n => !n.read).length;
  }, [notifications]);

  const contextValue: NotificationContextType = {
    notifications,
    preferences,
    showToast,
    dismissToast,
    markAsRead,
    markAllAsRead,
    clearNotification,
    clearAllNotifications,
    updatePreferences,
    unreadCount,
    isWebSocketConnected: isConnected,
    webSocketReconnectCount: reconnectCount,
  };

  return (
    <NotificationContext.Provider value={contextValue}>
      {children}
    </NotificationContext.Provider>
  );
};

export const useNotifications = (): NotificationContextType => {
  const context = useContext(NotificationContext);
  if (context === undefined) {
    throw new Error('useNotifications must be used within a NotificationProvider');
  }
  return context;
};