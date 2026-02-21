import { useEffect, useState } from 'react';

interface Alert {
  alert_type: 'SuccessRateDrop' | 'LatencyIncrease' | 'LiquidityDecrease';
  corridor_id: string;
  message: string;
  old_value: number;
  new_value: number;
  timestamp: string;
}

export default function AlertNotifications() {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [ws, setWs] = useState<WebSocket | null>(null);

  useEffect(() => {
    const websocket = new WebSocket('ws://localhost:8080/ws/alerts');
    
    websocket.onmessage = (event) => {
      const alert = JSON.parse(event.data);
      setAlerts((prev) => [alert, ...prev].slice(0, 10));
    };

    websocket.onerror = () => {
      setTimeout(() => setWs(null), 5000);
    };

    setWs(websocket);

    return () => websocket.close();
  }, []);

  const getAlertColor = (type: Alert['alert_type']) => {
    switch (type) {
      case 'SuccessRateDrop': return 'bg-red-100 border-red-500 text-red-900';
      case 'LatencyIncrease': return 'bg-yellow-100 border-yellow-500 text-yellow-900';
      case 'LiquidityDecrease': return 'bg-orange-100 border-orange-500 text-orange-900';
    }
  };

  const getAlertIcon = (type: Alert['alert_type']) => {
    switch (type) {
      case 'SuccessRateDrop': return '⚠️';
      case 'LatencyIncrease': return '⏱️';
      case 'LiquidityDecrease': return '💧';
    }
  };

  if (alerts.length === 0) return null;

  return (
    <div className="fixed top-4 right-4 z-50 space-y-2 max-w-md">
      {alerts.map((alert, idx) => (
        <div
          key={`${alert.timestamp}-${idx}`}
          className={`p-4 border-l-4 rounded shadow-lg ${getAlertColor(alert.alert_type)}`}
        >
          <div className="flex items-start">
            <span className="text-2xl mr-3">{getAlertIcon(alert.alert_type)}</span>
            <div className="flex-1">
              <p className="font-semibold text-sm">{alert.corridor_id}</p>
              <p className="text-sm mt-1">{alert.message}</p>
              <p className="text-xs mt-1 opacity-75">
                {new Date(alert.timestamp).toLocaleTimeString()}
              </p>
            </div>
            <button
              onClick={() => setAlerts((prev) => prev.filter((_, i) => i !== idx))}
              className="ml-2 text-lg opacity-50 hover:opacity-100"
            >
              ×
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
