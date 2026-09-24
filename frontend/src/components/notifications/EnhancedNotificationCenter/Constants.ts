import { CheckCircle, AlertCircle, AlertTriangle, Info } from 'lucide-react';
import type { NotificationType, NotificationPriority } from '@/types/notifications';

export const NOTIFICATION_ICONS: Record<NotificationType, typeof CheckCircle> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

export const TYPE_COLORS: Record<NotificationType, string> = {
  success: 'text-green-500',
  error: 'text-red-500',
  warning: 'text-yellow-500',
  info: 'text-blue-500',
};

export const PRIORITY_COLORS: Record<NotificationPriority, string> = {
  low: 'border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-900',
  medium: 'border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-900/20',
  high: 'border-orange-200 bg-orange-50 dark:border-orange-800 dark:bg-orange-900/20',
  critical: 'border-red-200 bg-red-50 dark:border-red-800 dark:bg-red-900/20',
};
