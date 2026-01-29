'use client';

import { CorridorAnalytics } from '@/lib/analytics-api';
import { TrendingUp } from 'lucide-react';

interface TopCorridorsProps {
  corridors: CorridorAnalytics[];
}

export function TopCorridors({ corridors }: TopCorridorsProps) {
  // Sort by success rate descending
  const sortedCorridors = [...corridors].sort(
    (a, b) => b.success_rate - a.success_rate
  );

  const getStatusColor = (successRate: number) => {
    if (successRate >= 99) return 'bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-300';
    if (successRate >= 97) return 'bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-300';
    if (successRate >= 95)
      return 'bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-300';
    return 'bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-300';
  };

  const formatCurrency = (value: number) => {
    if (value >= 1000000) {
      return `$${(value / 1000000).toFixed(2)}M`;
    }
    if (value >= 1000) {
      return `$${(value / 1000).toFixed(0)}K`;
    }
    return `$${value.toFixed(0)}`;
  };

  const getCorridorLabel = (corridor: CorridorAnalytics) => {
    return `${corridor.asset_a_code} → ${corridor.asset_b_code}`;
  };

  return (
    <div className="bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6">
      <h2 className="text-lg font-bold text-gray-900 dark:text-white mb-2">
        Top Corridors by Success Rate
      </h2>
      <p className="text-sm text-gray-600 dark:text-gray-400 mb-6">
        Highest performing payment corridors
      </p>

      <div className="space-y-3">
        {sortedCorridors.slice(0, 5).map((corridor, index) => (
          <div
            key={corridor.corridor_key}
            className="flex items-center justify-between p-4 bg-gray-50 dark:bg-slate-700 rounded-lg hover:bg-gray-100 dark:hover:bg-slate-600 transition-colors"
          >
            <div className="flex items-center gap-4 flex-1">
              <div className="flex items-center justify-center w-8 h-8 bg-blue-100 dark:bg-blue-900 rounded-full text-sm font-bold text-blue-700 dark:text-blue-300">
                {index + 1}
              </div>
              <div className="flex-1">
                <p className="font-medium text-gray-900 dark:text-white">
                  {getCorridorLabel(corridor)}
                </p>
                <div className="flex gap-4 mt-1 text-xs text-gray-600 dark:text-gray-400">
                  <span>Vol: {formatCurrency(corridor.volume_usd)}</span>
                  <span>Txns: {corridor.total_transactions}</span>
                  {corridor.avg_settlement_latency_ms && (
                    <span>Latency: {corridor.avg_settlement_latency_ms}ms</span>
                  )}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-4">
              <div>
                <div className="mb-2">
                  <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
                    Liquidity
                  </p>
                  <p className="text-sm font-semibold text-gray-900 dark:text-white">
                    {formatCurrency(corridor.liquidity_depth_usd)}
                  </p>
                </div>
              </div>

              <div className="text-right">
                <span
                  className={`inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm font-semibold ${getStatusColor(
                    corridor.success_rate
                  )}`}
                >
                  <TrendingUp className="w-4 h-4" />
                  {corridor.success_rate.toFixed(1)}%
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Stats Summary */}
      <div className="grid grid-cols-4 gap-4 mt-6 pt-6 border-t border-gray-200 dark:border-slate-700">
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            Total Volume
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatCurrency(
              sortedCorridors.reduce((sum, c) => sum + c.volume_usd, 0)
            )}
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            Avg Success Rate
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {(
              sortedCorridors.reduce((sum, c) => sum + c.success_rate, 0) /
              sortedCorridors.length
            ).toFixed(1)}
            %
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            Total Txns
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {sortedCorridors
              .reduce((sum, c) => sum + c.total_transactions, 0)
              .toLocaleString()}
          </p>
        </div>
        <div>
          <p className="text-xs text-gray-600 dark:text-gray-400 mb-1">
            Total Liquidity
          </p>
          <p className="text-lg font-bold text-gray-900 dark:text-white">
            {formatCurrency(
              sortedCorridors.reduce((sum, c) => sum + c.liquidity_depth_usd, 0)
            )}
          </p>
        </div>
      </div>
    </div>
  );
}
