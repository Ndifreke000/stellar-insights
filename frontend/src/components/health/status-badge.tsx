"use client";
import React from 'react';

interface StatusBadgeProps {
  status: string;
}

const StatusBadge: React.FC<StatusBadgeProps> = ({ status }) => {
  const getStatusColor = () => {
    switch (status.toLowerCase()) {
      case 'green':
        return 'bg-green-500';
      case 'yellow':
        return 'bg-yellow-500';
      case 'red':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  return (
    <span className={`px-2 py-1 text-xs font-semibold text-white rounded-full ${getStatusColor()}`}>
      {status}
    </span>
  );
};

export default StatusBadge;
