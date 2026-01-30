'use client';

import React, { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { X, CheckCircle, AlertCircle, AlertTriangle, Info } from 'lucide-react';
import { ToastNotification, NotificationType } from '@/types/notifications';

interface ToastProps {
  notification: ToastNotification;
  onDismiss: (id: string) => void;
  onAction?: (actionId: string, notificationId: string) => void;
}

const TOAST_ICONS: Record<NotificationType, React.ComponentType<{ className?: string }>> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const TOAST_STYLES: Record<NotificationType, string> = {
  success: 'bg-green-50 border-green-200 text-green-800 dark:bg-green-900/20 dark:border-green-800 dark:text-green-200',
  error: 'bg-red-50 border-red-200 text-red-800 dark:bg-red-900/20 dark:border-red-800 dark:text-red-200',
  warning: 'bg-yellow-50 border-yellow-200 text-yellow-800 dark:bg-yellow-900/20 dark:border-yellow-800 dark:text-yellow-200',
  info: 'bg-blue-50 border-blue-200 text-blue-800 dark:bg-blue-900/20 dark:border-blue-800 dark:text-blue-200',
};

const ICON_STYLES: Record<NotificationType, string> = {
  success: 'text-green-500 dark:text-green-400',
  error: 'text-red-500 dark:text-red-400',
  warning: 'text-yellow-500 dark:text-yellow-400',
  info: 'text-blue-500 dark:text-blue-400',
};

const PRIORITY_STYLES: Record<string, string> = {
  low: 'border-l-2',
  medium: 'border-l-4',
  high: 'border-l-4 shadow-lg',
  critical: 'border-l-4 shadow-xl ring-2 ring-red-500/20',
};

export const Toast: React.FC<ToastProps> = ({ 
  notification, 
  onDismiss, 
  onAction 
}) => {
  const [isVisible, setIsVisible] = useState(true);
  const [progress, setProgress] = useState(100);

  const Icon = TOAST_ICONS[notification.type];
  const duration = notification.duration || 5000;

  useEffect(() => {
    if (!notification.persistent && duration > 0) {
      const progressInterval = setInterval(() => {
        setProgress((prev) => {
          const newProgress = prev - (100 / (duration / 100));
          return Math.max(0, newProgress);
        });
      }, 100);

      const timer = setTimeout(() => {
        setIsVisible(false);
        setTimeout(() => onDismiss(notification.id), 300);
      }, duration);

      return () => {
        clearTimeout(timer);
        clearInterval(progressInterval);
      };
    }
  }, [notification.id, notification.persistent, duration, onDismiss]);

  const handleDismiss = () => {
    setIsVisible(false);
    setTimeout(() => onDismiss(notification.id), 300);
  };

  const handleActionClick = (actionId: string) => {
    onAction?.(actionId, notification.id);
  };

  return (
    <AnimatePresence>
      {isVisible && (
        <motion.div
          initial={{ opacity: 0, x: 300, scale: 0.9 }}
          animate={{ opacity: 1, x: 0, scale: 1 }}
          exit={{ opacity: 0, x: 300, scale: 0.9 }}
          transition={{ 
            type: "spring", 
            stiffness: 300, 
            damping: 30,
            mass: 0.8 
          }}
          className={`
            relative max-w-sm w-full border rounded-lg shadow-lg backdrop-blur-sm
            ${TOAST_STYLES[notification.type]}
            ${PRIORITY_STYLES[notification.priority]}
            overflow-hidden
          `}
          role="alert"
          aria-live={notification.priority === 'critical' ? 'assertive' : 'polite'}
        >
          {/* Progress bar for auto-dismiss */}
          {!notification.persistent && duration > 0 && (
            <div className="absolute top-0 left-0 h-1 bg-current opacity-20">
              <motion.div
                className="h-full bg-current"
                initial={{ width: '100%' }}
                animate={{ width: `${progress}%` }}
                transition={{ duration: 0.1, ease: 'linear' }}
              />
            </div>
          )}

          <div className="p-4">
            <div className="flex items-start">
              <div className="flex-shrink-0">
                <Icon className={`h-5 w-5 ${ICON_STYLES[notification.type]}`} />
              </div>
              
              <div className="ml-3 flex-1">
                <h3 className="text-sm font-medium">
                  {notification.title}
                </h3>
                <p className="mt-1 text-sm opacity-90">
                  {notification.message}
                </p>
                
                {/* Actions */}
                {notification.actions && notification.actions.length > 0 && (
                  <div className="mt-3 flex space-x-2">
                    {notification.actions.map((action) => (
                      <button
                        key={action.id}
                        onClick={() => handleActionClick(action.id)}
                        className={`
                          px-3 py-1 text-xs font-medium rounded-md transition-colors
                          ${action.variant === 'primary' 
                            ? 'bg-current text-white hover:opacity-90' 
                            : action.variant === 'destructive'
                            ? 'bg-red-600 text-white hover:bg-red-700'
                            : 'bg-white/20 hover:bg-white/30'
                          }
                        `}
                      >
                        {action.label}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Dismiss button */}
              {(notification.dismissible !== false) && (
                <div className="ml-4 flex-shrink-0">
                  <button
                    onClick={handleDismiss}
                    className="inline-flex rounded-md p-1.5 hover:bg-white/20 focus:outline-none focus:ring-2 focus:ring-current focus:ring-offset-2 transition-colors"
                    aria-label="Dismiss notification"
                  >
                    <X className="h-4 w-4" />
                  </button>
                </div>
              )}
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};