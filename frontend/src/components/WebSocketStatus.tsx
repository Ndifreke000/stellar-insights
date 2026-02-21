import React from 'react';

export interface WebSocketStatusProps {
  isConnected: boolean;
  isConnecting: boolean;
  connectionAttempts: number;
  onReconnect?: () => void;
  className?: string;
}

export function WebSocketStatus({
  isConnected,
  isConnecting,
  connectionAttempts,
  onReconnect,
  className = '',
}: WebSocketStatusProps) {
  const getStatusColor = () => {
    if (isConnected) return 'text-green-600 bg-green-100';
    if (isConnecting) return 'text-yellow-600 bg-yellow-100';
    return 'text-red-600 bg-red-100';
  };

  const getStatusText = () => {
    if (isConnected) return 'Connected';
    if (isConnecting) return 'Connecting...';
    return `Disconnected${connectionAttempts > 0 ? ` (${connectionAttempts} attempts)` : ''}`;
  };

  const getStatusIcon = () => {
    if (isConnected) {
      return (
        <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
          <circle cx="10" cy="10" r="6" />
        </svg>
      );
    }
    
    if (isConnecting) {
      return (
        <svg className="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
          />
          <path
            className="opacity-75"
            fill="currentColor"
            d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
          />
        </svg>
      );
    }

    return (
      <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
        <circle cx="10" cy="10" r="6" />
      </svg>
    );
  };

  return (
    <div className={`flex items-center space-x-2 ${className}`}>
      <div className={`flex items-center space-x-1 px-2 py-1 rounded-full text-xs font-medium ${getStatusColor()}`}>
        {getStatusIcon()}
        <span>{getStatusText()}</span>
      </div>
      
      {!isConnected && !isConnecting && onReconnect && (
        <button
          onClick={onReconnect}
          className="text-xs text-blue-600 hover:text-blue-800 underline"
        >
          Reconnect
        </button>
      )}
    </div>
  );
}