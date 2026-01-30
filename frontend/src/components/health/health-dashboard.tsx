"use client";
import React, { useEffect, useState } from 'react';
import { ResponsiveContainer, LineChart, Line, XAxis, YAxis, Tooltip } from 'recharts';
import { AlertTriangle, Settings, Activity } from 'lucide-react';
import { getAnchors, AnchorMetrics } from '../../lib/api';
import { MainLayout } from '../layout';
import StatusBadge from './status-badge';

interface AlertThreshold {
  healthScore: number;
  uptimePercentage: number;
  enabled: boolean;
}

interface FailureRecord {
  timestamp: string;
  reason: string;
  corridor?: string;
}

const generateHistoricalData = (currentScore: number) => {
  const data = [];
  for (let i = 29; i >= 0; i--) {
    const date = new Date(Date.now() - i * 24 * 60 * 60 * 1000);
    const variation = (Math.random() - 0.5) * 10;
    const score = Math.max(0, Math.min(100, currentScore + variation));
    data.push({
      date: date.toISOString().split('T')[0],
      score: Math.round(score * 10) / 10
    });
  }
  return data;
};

const generateRecentFailures = (): FailureRecord[] => {
  const reasons = ['Timeout', 'Insufficient liquidity', 'Path payment failed', 'Network congestion'];
  const corridors = ['USDC-PHP', 'EURC-NGN', 'USDT-KES', 'XLM-USD'];
  
  return Array.from({ length: Math.floor(Math.random() * 5) + 1 }, (_, i) => ({
    timestamp: new Date(Date.now() - Math.random() * 24 * 60 * 60 * 1000).toISOString(),
    reason: reasons[Math.floor(Math.random() * reasons.length)],
    corridor: corridors[Math.floor(Math.random() * corridors.length)]
  })).sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
};

const HealthDashboard = () => {
  const [anchors, setAnchors] = useState<AnchorMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [alertThresholds, setAlertThresholds] = useState<AlertThreshold>({
    healthScore: 85,
    uptimePercentage: 95,
    enabled: true
  });
  const [showSettings, setShowSettings] = useState(false);

  useEffect(() => {
    const fetchAnchors = async () => {
      try {
        const response = await getAnchors();
        setAnchors(response.anchors);
      } catch (err) {
        setError('Failed to fetch anchor data.');
        console.error(err);
      } finally {
        setLoading(false);
      }
    };

    fetchAnchors();
  }, []);

  const getHealthStatus = (score: number): 'green' | 'yellow' | 'red' => {
    if (score >= 95) return 'green';
    if (score >= 85) return 'yellow';
    return 'red';
  };

  const calculateUptime = (anchor: AnchorMetrics): number => {
    return anchor.total_transactions > 0 
      ? (anchor.successful_transactions / anchor.total_transactions) * 100
      : 0;
  };

  const formatTimestamp = (timestamp: string): string => {
    return new Date(timestamp).toLocaleString();
  };

  if (loading) {
    return (
      <MainLayout>
        <div className="flex items-center justify-center min-h-screen">
          <div className="text-center">
            <Activity className="w-12 h-12 text-gray-400 mx-auto mb-4 animate-pulse" />
            <p className="text-gray-600 dark:text-gray-400">Loading anchor health data...</p>
          </div>
        </div>
      </MainLayout>
    );
  }

  if (error) {
    return (
      <MainLayout>
        <div className="flex items-center justify-center min-h-screen">
          <div className="text-center">
            <AlertTriangle className="w-12 h-12 text-red-500 mx-auto mb-4" />
            <p className="text-red-500">{error}</p>
          </div>
        </div>
      </MainLayout>
    );
  }

  const healthyAnchors = anchors.filter(a => getHealthStatus(a.reliability_score) === 'green').length;
  const warningAnchors = anchors.filter(a => getHealthStatus(a.reliability_score) === 'yellow').length;
  const criticalAnchors = anchors.filter(a => getHealthStatus(a.reliability_score) === 'red').length;

  return (
    <MainLayout>
      <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
        <div className="mb-8">
          <div className="flex justify-between items-center mb-4">
            <div>
              <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-2 flex items-center gap-2">
                <Activity className="w-8 h-8 text-blue-500" />
                Anchor Health Dashboard
              </h1>
              <p className="text-gray-600 dark:text-gray-400">
                Real-time monitoring of anchor reliability and performance metrics
              </p>
            </div>
            <button
              onClick={() => setShowSettings(!showSettings)}
              className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              <Settings className="w-4 h-4" />
              Alerts
            </button>
          </div>

          {/* Summary Stats */}
          <div className="grid grid-cols-1 sm:grid-cols-4 gap-4 mb-8">
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">Total Anchors</div>
              <div className="text-2xl font-bold text-gray-900 dark:text-white">{anchors.length}</div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">Healthy</div>
              <div className="text-2xl font-bold text-green-600">{healthyAnchors}</div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">Warning</div>
              <div className="text-2xl font-bold text-yellow-600">{warningAnchors}</div>
            </div>
            <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-4">
              <div className="text-sm text-gray-500 dark:text-gray-400 mb-1">Critical</div>
              <div className="text-2xl font-bold text-red-600">{criticalAnchors}</div>
            </div>
          </div>
        </div>

        {/* Alert Configuration */}
        {showSettings && (
          <div className="mb-8 bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
            <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4">Alert Threshold Configuration</h2>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Health Score Threshold (%)
                </label>
                <input
                  type="number"
                  value={alertThresholds.healthScore}
                  onChange={(e) => setAlertThresholds(prev => ({ ...prev, healthScore: Number(e.target.value) }))}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-slate-600 rounded-lg bg-white dark:bg-slate-700 text-gray-900 dark:text-white"
                  min="0"
                  max="100"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                  Uptime Threshold (%)
                </label>
                <input
                  type="number"
                  value={alertThresholds.uptimePercentage}
                  onChange={(e) => setAlertThresholds(prev => ({ ...prev, uptimePercentage: Number(e.target.value) }))}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-slate-600 rounded-lg bg-white dark:bg-slate-700 text-gray-900 dark:text-white"
                  min="0"
                  max="100"
                />
              </div>
              <div className="flex items-end">
                <label className="flex items-center">
                  <input
                    type="checkbox"
                    checked={alertThresholds.enabled}
                    onChange={(e) => setAlertThresholds(prev => ({ ...prev, enabled: e.target.checked }))}
                    className="mr-2"
                  />
                  <span className="text-sm text-gray-700 dark:text-gray-300">Enable Alerts</span>
                </label>
              </div>
            </div>
          </div>
        )}
        
        {/* Anchor Health Cards */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {anchors.map((anchor) => {
            const uptime = calculateUptime(anchor);
            const historicalData = generateHistoricalData(anchor.reliability_score);
            const recentFailures = generateRecentFailures();
            const healthStatus = getHealthStatus(anchor.reliability_score);
            
            return (
              <div key={anchor.id} className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6 hover:shadow-lg transition-shadow">
                <div className="flex justify-between items-start mb-6">
                  <div>
                    <h2 className="text-xl font-bold text-gray-900 dark:text-white">{anchor.name}</h2>
                    <p className="text-sm text-gray-500 dark:text-gray-400 font-mono">
                      {anchor.stellar_account.slice(0, 8)}...{anchor.stellar_account.slice(-8)}
                    </p>
                  </div>
                  <StatusBadge status={healthStatus} />
                </div>
                
                <div className="grid grid-cols-2 gap-4 mb-6">
                  <div className="p-4 bg-gray-50 dark:bg-slate-700 rounded-lg">
                    <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Health Score</h3>
                    <div className="flex items-center gap-2">
                      <span className="text-3xl font-bold text-gray-900 dark:text-white">
                        {anchor.reliability_score.toFixed(1)}%
                      </span>
                      <div className={`w-3 h-3 rounded-full ${
                        healthStatus === 'green' ? 'bg-green-500' :
                        healthStatus === 'yellow' ? 'bg-yellow-500' : 'bg-red-500'
                      }`} />
                    </div>
                  </div>
                  <div className="p-4 bg-gray-50 dark:bg-slate-700 rounded-lg">
                    <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Uptime</h3>
                    <div className="text-3xl font-bold text-gray-900 dark:text-white">
                      {uptime.toFixed(1)}%
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                      {anchor.successful_transactions.toLocaleString()}/{anchor.total_transactions.toLocaleString()} transactions
                    </div>
                  </div>
                </div>

                {/* Historical Health Chart */}
                <div className="mb-6">
                  <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">30-Day Health Trend</h3>
                  <div className="h-32">
                    <ResponsiveContainer width="100%" height="100%">
                      <LineChart data={historicalData}>
                        <XAxis 
                          dataKey="date" 
                          tick={false}
                          axisLine={false}
                        />
                        <YAxis 
                          domain={[0, 100]}
                          tick={false}
                          axisLine={false}
                        />
                        <Tooltip 
                          labelFormatter={(value) => `Date: ${value}`}
                          formatter={(value: number) => [`${value}%`, 'Health Score']}
                        />
                        <Line
                          type="monotone"
                          dataKey="score"
                          stroke={healthStatus === 'green' ? '#10b981' : healthStatus === 'yellow' ? '#f59e0b' : '#ef4444'}
                          strokeWidth={2}
                          dot={false}
                        />
                      </LineChart>
                    </ResponsiveContainer>
                  </div>
                </div>

                {/* Recent Failures */}
                <div>
                  <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">Recent Failures</h3>
                  <div className="space-y-2 max-h-32 overflow-y-auto">
                    {recentFailures.length > 0 ? (
                      recentFailures.map((failure, index) => (
                        <div key={index} className="flex justify-between items-center p-2 bg-red-50 dark:bg-red-900/20 rounded text-xs">
                          <div>
                            <span className="font-medium text-red-800 dark:text-red-400">{failure.reason}</span>
                            {failure.corridor && (
                              <span className="text-red-600 dark:text-red-500 ml-2">({failure.corridor})</span>
                            )}
                          </div>
                          <span className="text-red-600 dark:text-red-500">
                            {formatTimestamp(failure.timestamp).split(',')[1]?.trim()}
                          </span>
                        </div>
                      ))
                    ) : (
                      <div className="p-3 bg-green-50 dark:bg-green-900/20 rounded text-xs text-green-700 dark:text-green-400 text-center">
                        No recent failures
                      </div>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </MainLayout>
  );
};

export default HealthDashboard;
