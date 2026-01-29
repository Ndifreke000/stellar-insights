'use client';

import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Wifi, WifiOff, RotateCw } from 'lucide-react';
import { useNotifications } from '@/contexts/NotificationContext';

interface ConnectionStatusProps {
  className?: string;
  showLabel?: boolean;
  size?: 'sm' | 'md' | 'lg';
}

export const ConnectionStatus: React.FC<ConnectionStatusProps> = ({ 
  className = '', 
  showLabel = true,
  size = 'md'
}) => {
  const { isWebSocketConnected, webSocketReconnectCount } = useNotifications();

  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-5 h-5',
    lg: 'w-6 h-6',
  };

  const textSizeClasses = {
    sm: 'text-xs',
    md: 'text-sm',
    lg: 'text-base',
  };

  const getStatusInfo = () => {
    if (webSocketReconnectCount > 0) {
      return {
        icon: RotateCw,
        label: 'Reconnecting...',
        color: 'text-yellow-500',
        bgColor: 'bg-yellow-100 dark:bg-yellow-900/20',
        borderColor: 'border-yellow-200 dark:border-yellow-800',
        animate: true,
      };
    }

    if (isWebSocketConnected) {
      return {
        icon: Wifi,
        label: 'Connected',
        color: 'text-green-500',
        bgColor: 'bg-green-100 dark:bg-green-900/20',
        borderColor: 'border-green-200 dark:border-green-800',
        animate: false,
      };
    }

    return {
      icon: WifiOff,
      label: 'Disconnected',
      color: 'text-gray-400',
      bgColor: 'bg-gray-100 dark:bg-gray-800',
      borderColor: 'border-gray-200 dark:border-gray-700',
      animate: false,
    };
  };

  const status = getStatusInfo();
  const Icon = status.icon;

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      <div className={`
        flex items-center justify-center rounded-full p-1.5 border
        ${status.bgColor} ${status.borderColor}
      `}>
        <motion.div
          animate={status.animate ? { rotate: 360 } : {}}
          transition={status.animate ? { duration: 1, repeat: Infinity, ease: 'linear' } : {}}
        >
          <Icon className={`${sizeClasses[size]} ${status.color}`} />
        </motion.div>
      </div>
      
      {showLabel && (
        <AnimatePresence mode="wait">
          <motion.span
            key={status.label}
            initial={{ opacity: 0, x: -10 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 10 }}
            className={`font-medium ${status.color} ${textSizeClasses[size]}`}
          >
            {status.label}
            {webSocketReconnectCount > 0 && ` (${webSocketReconnectCount}/5)`}
          </motion.span>
        </AnimatePresence>
      )}
    </div>
  );
};