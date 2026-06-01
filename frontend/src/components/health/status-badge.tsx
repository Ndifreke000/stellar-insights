"use client";
import React from 'react';

interface StatusBadgeProps {
  status: 'green' | 'yellow' | 'red' | string;
}

const StatusBadge: React.FC<StatusBadgeProps> = ({ status }) => {
  const getStatusConfig = (status: string) => {
    const normalizedStatus = status.toLowerCase();
    
    if (normalizedStatus === 'green' || normalizedStatus === 'healthy') {
      return {
        bgColor: 'bg-green-100 dark:bg-green-900/20',
        textColor: 'text-green-800 dark:text-green-400',
        dotColor: 'bg-green-500',
        label: 'Healthy'
      };
    } else if (normalizedStatus === 'yellow' || normalizedStatus === 'warning') {
      return {
        bgColor: 'bg-yellow-100 dark:bg-yellow-900/20',
        textColor: 'text-yellow-800 dark:text-yellow-400',
        dotColor: 'bg-yellow-500',
        label: 'Warning'
      };
    } else {
      return {
        bgColor: 'bg-red-100 dark:bg-red-900/20',
        textColor: 'text-red-800 dark:text-red-400',
        dotColor: 'bg-red-500',
        label: 'Critical'
      };
    }
  };

  const config = getStatusConfig(status);

  return (
    <span className={`inline-flex items-center gap-2 px-3 py-1 rounded-full text-sm font-medium ${config.bgColor} ${config.textColor}`}>
      <div className={`w-2 h-2 rounded-full ${config.dotColor}`} />
      {config.label}
    </span>
  );
};

export default StatusBadge;
