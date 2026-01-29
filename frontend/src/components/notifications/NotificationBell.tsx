'use client';

import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Bell, Settings } from 'lucide-react';
import { useNotifications } from '@/contexts/NotificationContext';
import { NotificationHistory } from './NotificationHistory';
import { NotificationPreferences } from './NotificationPreferences';

export const NotificationBell: React.FC = () => {
  const { unreadCount } = useNotifications();
  const [showHistory, setShowHistory] = useState(false);
  const [showPreferences, setShowPreferences] = useState(false);

  return (
    <>
      <div className="relative">
        {/* Notification Bell Button */}
        <button
          onClick={() => setShowHistory(true)}
          className="relative p-2 text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
          aria-label={`Notifications ${unreadCount > 0 ? `(${unreadCount} unread)` : ''}`}
        >
          <Bell className="h-6 w-6" />
          
          {/* Unread Count Badge */}
          <AnimatePresence>
            {unreadCount > 0 && (
              <motion.span
                initial={{ scale: 0, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                exit={{ scale: 0, opacity: 0 }}
                className="absolute -top-1 -right-1 bg-red-500 text-white text-xs font-bold rounded-full h-5 w-5 flex items-center justify-center min-w-[20px]"
              >
                {unreadCount > 99 ? '99+' : unreadCount}
              </motion.span>
            )}
          </AnimatePresence>

          {/* Pulse Animation for New Notifications */}
          {unreadCount > 0 && (
            <motion.div
              className="absolute inset-0 rounded-lg bg-red-500 opacity-20"
              animate={{ scale: [1, 1.1, 1] }}
              transition={{ duration: 2, repeat: Infinity }}
            />
          )}
        </button>

        {/* Settings Button */}
        <button
          onClick={() => setShowPreferences(true)}
          className="ml-1 p-2 text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
          aria-label="Notification settings"
        >
          <Settings className="h-5 w-5" />
        </button>
      </div>

      {/* Notification History Modal */}
      <NotificationHistory
        isOpen={showHistory}
        onClose={() => setShowHistory(false)}
      />

      {/* Notification Preferences Modal */}
      <NotificationPreferences
        isOpen={showPreferences}
        onClose={() => setShowPreferences(false)}
      />
    </>
  );
};