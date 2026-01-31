'use client';

import React, { useState, useEffect, useMemo } from 'react';
import { ToastContainer } from './ToastContainer';
import { useNotifications } from '@/contexts/NotificationContext';
import { ToastNotification } from '@/types/notifications';

export const NotificationSystem: React.FC = () => {
  const { notifications, dismissToast, preferences } = useNotifications();
  const [isClient, setIsClient] = useState(false);
  const [now, setNow] = useState(() => Date.now());

  // Only render on client side to avoid hydration mismatch
  useEffect(() => {
    // Use requestAnimationFrame to defer state update and avoid cascading renders
    const id = requestAnimationFrame(() => setIsClient(true));
    return () => cancelAnimationFrame(id);
  }, []);

  // Update time periodically to filter old notifications
  useEffect(() => {
    const interval = setInterval(() => {
      setNow(Date.now());
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  // Convert BaseNotifications to ToastNotifications for display
  // Always call useMemo unconditionally to follow rules of hooks
  const toastNotifications: ToastNotification[] = useMemo(() =>
    notifications
      .filter(n =>
        // Show notifications that are less than 30 seconds old or persistent
        n.persistent || (now - new Date(n.timestamp).getTime()) < 30000
      )
      .map(n => ({
        ...n,
        duration: preferences.autoHideDelay,
        dismissible: true,
        position: 'top-right' as const,
      })),
    [notifications, preferences.autoHideDelay, now]
  );

  // Don't render anything during SSR
  if (!isClient) {
    return null;
  }

  return (
    <ToastContainer
      notifications={toastNotifications}
      onDismiss={dismissToast}
      position="top-right"
      maxNotifications={5}
    />
  );
};