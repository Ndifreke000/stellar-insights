'use client';

import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Toast } from './Toast';
import { ToastNotification } from '@/types/notifications';

interface ToastContainerProps {
  notifications: ToastNotification[];
  position?: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left' | 'top-center' | 'bottom-center';
  onDismiss: (id: string) => void;
  onAction?: (actionId: string, notificationId: string) => void;
  maxNotifications?: number;
}

const POSITION_STYLES: Record<string, string> = {
  'top-right': 'top-4 right-4',
  'top-left': 'top-4 left-4',
  'bottom-right': 'bottom-4 right-4',
  'bottom-left': 'bottom-4 left-4',
  'top-center': 'top-4 left-1/2 transform -translate-x-1/2',
  'bottom-center': 'bottom-4 left-1/2 transform -translate-x-1/2',
};

const CONTAINER_DIRECTION: Record<string, string> = {
  'top-right': 'flex-col',
  'top-left': 'flex-col',
  'bottom-right': 'flex-col-reverse',
  'bottom-left': 'flex-col-reverse',
  'top-center': 'flex-col',
  'bottom-center': 'flex-col-reverse',
};

export const ToastContainer: React.FC<ToastContainerProps> = ({
  notifications,
  position = 'top-right',
  onDismiss,
  onAction,
  maxNotifications = 5,
}) => {
  // Sort notifications by priority and timestamp
  const sortedNotifications = [...notifications]
    .sort((a, b) => {
      const priorityOrder = { critical: 4, high: 3, medium: 2, low: 1 };
      const priorityDiff = priorityOrder[b.priority] - priorityOrder[a.priority];
      if (priorityDiff !== 0) return priorityDiff;
      return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
    })
    .slice(0, maxNotifications);

  if (sortedNotifications.length === 0) {
    return null;
  }

  return (
    <div
      className={`
        fixed z-50 pointer-events-none
        ${POSITION_STYLES[position]}
      `}
      role="region"
      aria-label="Notifications"
    >
      <div className={`flex ${CONTAINER_DIRECTION[position]} space-y-2 pointer-events-auto`}>
        <AnimatePresence mode="popLayout">
          {sortedNotifications.map((notification, index) => (
            <motion.div
              key={notification.id}
              layout
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ 
                opacity: 1, 
                scale: 1,
                transition: { delay: index * 0.1 }
              }}
              exit={{ 
                opacity: 0, 
                scale: 0.8,
                transition: { duration: 0.2 }
              }}
            >
              <Toast
                notification={notification}
                onDismiss={onDismiss}
                onAction={onAction}
              />
            </motion.div>
          ))}
        </AnimatePresence>
        
        {/* Show overflow indicator */}
        {notifications.length > maxNotifications && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="text-center py-2"
          >
            <span className="text-xs text-gray-500 dark:text-gray-400 bg-white/80 dark:bg-gray-800/80 px-2 py-1 rounded-full backdrop-blur-sm">
              +{notifications.length - maxNotifications} more notifications
            </span>
          </motion.div>
        )}
      </div>
    </div>
  );
};