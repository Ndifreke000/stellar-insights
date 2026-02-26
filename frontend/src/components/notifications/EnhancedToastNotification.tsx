'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { motion } from 'framer-motion';
import { X, CheckCircle, AlertCircle, AlertTriangle, Info, ExternalLink } from 'lucide-react';
import { BaseNotification, NotificationType, NotificationPriority } from '@/types/notifications';
import { useNotifications } from '@/contexts/NotificationContext';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';

interface ToastNotificationProps {
  notification: BaseNotification;
  onClose: () => void;
}

const NOTIFICATION_ICONS: Record<NotificationType, React.FC<{ className?: string }>> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const TYPE_COLORS: Record<NotificationType, string> = {
  success: 'text-green-500',
  error: 'text-red-500',
  warning: 'text-yellow-500',
  info: 'text-blue-500',
};

const PRIORITY_COLORS: Record<NotificationPriority, string> = {
  low: 'border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-900',
  medium: 'border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-900/20',
  high: 'border-orange-200 bg-orange-50 dark:border-orange-800 dark:bg-orange-900/20',
  critical: 'border-red-200 bg-red-50 dark:border-red-800 dark:bg-red-900/20',
};

const AUTO_HIDE_DELAY: Record<NotificationPriority, number> = {
  low: 8000,
  medium: 6000,
  high: 4000,
  critical: 0, // Don't auto-hide critical notifications
};

export const EnhancedToastNotification: React.FC<ToastNotificationProps> = ({
  notification,
  onClose,
}) => {
  const [isVisible, setIsVisible] = useState(false);
  const [progress, setProgress] = useState(100);
  const { markAsRead } = useNotifications();
  const autoHideDelay = AUTO_HIDE_DELAY[notification.priority];
  let autoHideTimer: ReturnType<typeof setTimeout> | null = null;
  let progressTimer: ReturnType<typeof setInterval> | null = null;

  // Auto-hide logic
  useEffect(() => {
    setIsVisible(true);

    if (autoHideDelay > 0) {
      // Start progress animation
      const progressInterval = 50; // Update every 50ms
      const totalSteps = autoHideDelay / progressInterval;
      let currentStep = 0;

      progressTimer = setInterval(() => {
        currentStep++;
        const newProgress = Math.max(0, 100 - (currentStep / totalSteps) * 100);
        setProgress(newProgress);

        if (currentStep >= totalSteps) {
          handleClose();
        }
      }, progressInterval);

      autoHideTimer = setTimeout(() => {
        handleClose();
      }, autoHideDelay);
    }

    return () => {
      if (autoHideTimer) clearTimeout(autoHideTimer);
      if (progressTimer) clearInterval(progressTimer);
    };
  }, [notification.id, autoHideDelay]);

  const handleClose = useCallback(() => {
    setIsVisible(false);
    if (autoHideTimer) clearTimeout(autoHideTimer);
    if (progressTimer) clearInterval(progressTimer);
    setTimeout(onClose, 300); // Wait for exit animation
  }, [onClose]);

  const handleAction = useCallback((action: () => void | Promise<void>) => {
    action();
    handleClose();
  }, [handleClose]);

  const handleMarkAsRead = useCallback(() => {
    markAsRead(notification.id);
  }, [markAsRead, notification.id]);

  const IconComponent = NOTIFICATION_ICONS[notification.type];
  const priorityColor = PRIORITY_COLORS[notification.priority];
  const typeColor = TYPE_COLORS[notification.type];

  if (!isVisible) return null;

  return (
    <motion.div
      initial={{ opacity: 0, y: -50, scale: 0.3 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: -50, scale: 0.3 }}
      transition={{
        type: "spring",
        stiffness: 300,
        damping: 30,
        mass: 0.8
      }}
      className={`
        relative w-96 p-4 rounded-lg border shadow-lg backdrop-blur-sm
        ${priorityColor}
        ${notification.priority === 'critical' ? 'animate-pulse' : ''}
      `}
      style={{
        backdropFilter: 'blur(8px)',
      }}
    >
      {/* Progress Bar for Auto-hide */}
      {autoHideDelay > 0 && (
        <div className="absolute top-0 left-0 right-0 h-1 bg-gray-200 dark:bg-gray-700 rounded-t-lg overflow-hidden">
          <motion.div
            className="h-full bg-blue-500"
            initial={{ width: "100%" }}
            animate={{ width: `${progress}%` }}
            transition={{ duration: 0.05 }}
          />
        </div>
      )}

      {/* Close Button */}
      <button
        onClick={handleClose}
        className="absolute top-2 right-2 p-1 rounded-full hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
      >
        <X className="h-4 w-4 text-gray-500" />
      </button>

      {/* Notification Content */}
      <div className="flex items-start gap-3">
        {/* Icon */}
        <div className="shrink-0 mt-0.5">
          <IconComponent className={`h-5 w-5 ${typeColor}`} />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          {/* Header */}
          <div className="flex items-center gap-2 mb-2">
            <h4 className="font-semibold text-gray-900 dark:text-white truncate">
              {notification.title}
            </h4>
            <Badge variant="secondary" className="text-xs">
              {notification.priority}
            </Badge>
          </div>

          {/* Message */}
          <p className="text-sm text-gray-600 dark:text-gray-300 line-clamp-2 mb-3">
            {notification.message}
          </p>

          {/* Actions */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              {notification.actions?.map(action => (
                <Button
                  key={action.id}
                  variant={action.variant === 'primary' ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => handleAction(action.onClick)}
                  className="text-xs"
                >
                  {action.label}
                </Button>
              ))}

              <Button
                variant="ghost"
                size="sm"
                onClick={handleMarkAsRead}
                className="text-xs"
              >
                Mark as read
              </Button>
            </div>

            {/* Metadata */}
            {notification.metadata?.url && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  window.open(notification.metadata.url as string, '_blank');
                  handleMarkAsRead();
                }}
                className="text-xs"
              >
                <ExternalLink className="h-3 w-3" />
              </Button>
            )}
          </div>
        </div>
      </div>

      {/* Priority Indicator */}
      {notification.priority === 'critical' && (
        <div className="absolute -left-1 top-0 bottom-0 w-1 bg-red-500 rounded-l-lg animate-pulse" />
      )}
    </motion.div>
  );
};

// Toast Container Component
interface ToastContainerProps {
  notifications: BaseNotification[];
  onRemove: (id: string) => void;
}

export const EnhancedToastContainer: React.FC<ToastContainerProps> = ({
  notifications,
  onRemove,
}) => {
  return (
    <div className="fixed top-4 right-4 z-50 space-y-2 pointer-events-none">
      {notifications.map((notification, index) => (
        <motion.div
          key={notification.id}
          initial={{ opacity: 0, x: 100 }}
          animate={{ opacity: 1, x: 0 }}
          exit={{ opacity: 0, x: 100 }}
          transition={{
            delay: index * 0.1,
            type: "spring",
            stiffness: 300,
            damping: 30
          }}
          className="pointer-events-auto"
          style={{
            // Stack notifications with slight overlap
            transform: `translateY(${index * -10}px)`,
            zIndex: 1000 - index,
          }}
        >
          <EnhancedToastNotification
            notification={notification}
            onClose={() => onRemove(notification.id)}
          />
        </motion.div>
      ))}
    </div>
  );
};
