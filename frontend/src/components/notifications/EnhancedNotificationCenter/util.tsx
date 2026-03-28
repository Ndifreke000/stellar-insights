import { NotificationPriority, NotificationType } from '@/types/notifications';
import {
  Settings,
  CheckCircle,
  AlertCircle,
  AlertTriangle,
  Info,
  Clock,
  TrendingUp,
  Activity,
  RefreshCw,
  Zap,
  Shield,
  Globe,
  Database,
  Cpu,
  Wifi,
  Camera
} from 'lucide-react';

const NOTIFICATION_ICONS: Record<NotificationType, React.ComponentType<{ className?: string }>> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

const PRIORITY_ICONS: Record<NotificationPriority, React.ComponentType<{ className?: string }>> = {
  low: Clock,
  medium: Activity,
  high: TrendingUp,
  critical: Zap,
};

const PRIORITY_COLORS: Record<NotificationPriority, string> = {
  low: 'bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200',
  medium: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200',
  high: 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200',
  critical: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
};

const TYPE_COLORS: Record<NotificationType, string> = {
  success: 'text-green-500',
  error: 'text-red-500',
  warning: 'text-yellow-500',
  info: 'text-blue-500',
};

const CATEGORY_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  payments: Globe,
  liquidity: Database,
  snapshots: Camera,
  system: Cpu,
  security: Shield,
  network: Wifi,
  maintenance: Settings,
  updates: RefreshCw,
};

export {
  NOTIFICATION_ICONS,
  PRIORITY_ICONS,
  PRIORITY_COLORS,
  TYPE_COLORS,
  CATEGORY_ICONS,
}