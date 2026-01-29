'use client';

import React, { useState, useEffect } from 'react';
import { ToastContainer } from './ToastContainer';
import { useNotifications } from '@/contexts/NotificationContext';
import { ToastNotification } from '@/types/notifications';

export const NotificationSystem: React.FC = () => {
  const { notifications, dismissToast, preferences } = useNotifications();
  const [isClient, setIsClient] = useState(false);
  
  // Only render on client side to avoid hydration mismatch
  useEffect(() => {
    setIsClient(true);
  }, []);

  // Don't render anything during SSR
  if (!isClient) {
    return null;
  }
  
  // Convert BaseNotifications to ToastNotifications for display
  const toastNotifications: ToastNotification[] = notifications
    .filter(n => 
      // Show notifications that are less than 30 seconds old or persistent
      n.persistent || (Date.now() - new Date(n.timestamp).getTime()) < 30000
    )
    .map(n => ({
      ...n,
      duration: preferences.autoHideDelay,
      dismissible: true,
      position: 'top-right' as const,
    }));

  return (
    <ToastContainer
      notifications={toastNotifications}
      onDismiss={dismissToast}
      position="top-right"
      maxNotifications={5}
    />
  );
};