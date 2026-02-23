import React from 'react';
import { Metadata } from 'next';
import { NotificationCenterDemo } from '@/components/notifications';

export const metadata: Metadata = {
  title: 'Notification Center Demo - Stellar Insights',
  description: 'Interactive demo of the real-time notification system',
};

export default function NotificationCenterDemoPage() {
  return React.createElement(
    'div',
    { className: 'min-h-screen bg-gray-50 dark:bg-gray-950' },
    React.createElement(
      'div',
      { className: 'container mx-auto py-8' },
      React.createElement(NotificationCenterDemo)
    )
  );
}
