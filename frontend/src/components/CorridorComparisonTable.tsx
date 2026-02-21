import React, { useState } from 'react';
import { 
  TrendingUp, 
  TrendingDown, 
  Minus, 
  Download,
  Trophy,
  AlertTriangle
} from 'lucide-react';
import { CorridorMetrics } from '@/lib/api';

export interface CorridorComparisonTableProps {
  corridors: CorridorMetrics[];
  onExport?: () => void;
}

type MetricKey = keyof Pick<
  CorridorMetrics,
  | 'success_rate'
  | 'average_latency_ms'
  | 'liquidity_depth_usd'
  | 'liquidity_volume_24h_usd'
  | 'average_slippage_bps'
  | 'health_score'
>;

interface MetricConfig {
  label: string;
  key: MetricKey;
  format: (value: number) => string;
  higherIsBetter: boolean;
  unit?: string;
}

const METRICS: MetricConfig[] = [
  {
    label: 'Success Rate',
    key: 'success_rate',
    format: (v) => `${v.toFixed(2)}%`,
    higherIsBetter: true,
  },
  {
    label: 'Health Score',
    key: 'health_score',
    format: (v) => v.toFixed(1),
    higherIsBetter: true,
  },
  {
    label: 'Avg Latency',
    key: 'average_latency_ms',
    format: (v) => `${v.toFixed(0)}ms`,
    higherIsBetter: false,
  },
  {
    label: 'Liquidity Depth',
    key: 'liquidity_depth_usd',
    format: (v) => `$${(v / 1000000).toFixed(2)}M`,
    higherIsBetter: true,
  },
  {
    label: '24h Volume',
    key: 'liquidity_volume_24h_usd',
    format: (v) => `$${(v / 1000).toFixed(0)}K`,
    higherIsBetter: true,
  },
  {
    label: 'Avg Slippage',
    key: 'average_slippage_bps',
    format: (v) => `${v.toFixed(2)} bps`,
    higherIsBetter: false,
  },
];

export function CorridorComparisonTable({ 
  corridors, 
  onExport 
}: CorridorComparisonTableProps) {
  const [sortBy, setSortBy] = useState<MetricKey>('success_rate');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');

  // Find best and worst for each metric
  const getBestWorst = (metric: MetricConfig) => {
    const values = corridors.map((c) => c[metric.key] as number);
    const best = metric.higherIsBetter ? Math.max(...values) : Math.min(...values);
    const worst = metric.higherIsBetter ? Math.min(...values) : Math.max(...values);
    return { best, worst };
  };

  // Get indicator for value comparison
  const getIndicator = (value: number, metric: MetricConfig) => {
    const { best, worst } = getBestWorst(metric);
    
    if (value === best) {
      return <Trophy className="w-4 h-4 text-yellow-500" title="Best" />;
    }
    if (value === worst) {
      return <AlertTriangle className="w-4 h-4 text-red-500" title="Worst" />;
    }
    return null;
  };

  // Get trend indicator
  const getTrendIndicator = (value: number, metric: MetricConfig) => {
    const { best, worst } = getBestWorst(metric);
    const range = Math.abs(best - worst);
    const position = Math.abs(value - worst) / range;

    if (position > 0.8) {
      return <TrendingUp className="w-4 h-4 text-green-500" />;
    }
    if (position < 0.3) {
      return <TrendingDown className="w-4 h-4 text-red-500" />;
    }
    return <Minus className="w-4 h-4 text-gray-400" />;
  };

  // Get cell background color based on performance
  const getCellColor = (value: number, metric: MetricConfig) => {
    const { best, worst } = getBestWorst(metric);
    
    if (value === best) {
      return 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-700';
    }
    if (value === worst) {
      return 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-700';
    }
    return 'bg-white dark:bg-slate-800';
  };

  const handleSort = (key: MetricKey) => {
    if (sortBy === key) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortBy(key);
      setSortOrder('desc');
    }
  };

  const sortedCorridors = [...corridors].sort((a, b) => {
    const aVal = a[sortBy] as number;
    const bVal = b[sortBy] as number;
    return sortOrder === 'asc' ? aVal - bVal : bVal - aVal;
  });

  return (
    <div className="bg-white dark:bg-slate-800 rounded-lg shadow-lg border border-gray-200 dark:border-slate-700 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-slate-700">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
          Detailed Comparison
        </h3>
        {onExport && (
          <button
            onClick={onExport}
            className="flex items-center gap-2 px-3 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          >
            <Download className="w-4 h-4" />
            Export CSV
          </button>
        )}
      </div>

      {/* Table */}
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead className="bg-gray-50 dark:bg-slate-900">
            <tr>
              <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider sticky left-0 bg-gray-50 dark:bg-slate-900 z-10">
                Corridor
              </th>
              {METRICS.map((metric) => (
                <th
                  key={metric.key}
                  className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-800 transition-colors"
                  onClick={() => handleSort(metric.key)}
                >
                  <div className="flex items-center gap-2">
                    {metric.label}
                    {sortBy === metric.key && (
                      sortOrder === 'asc' ? 
                        <TrendingUp className="w-3 h-3" /> : 
                        <TrendingDown className="w-3 h-3" />
                    )}
                  </div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-200 dark:divide-slate-700">
            {sortedCorridors.map((corridor, idx) => (
              <tr
                key={corridor.id}
                className={`hover:bg-gray-50 dark:hover:bg-slate-700/50 transition-colors ${
                  idx % 2 === 0 ? 'bg-white dark:bg-slate-800' : 'bg-gray-50/50 dark:bg-slate-800/50'
                }`}
              >
                <td className="px-4 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white sticky left-0 bg-inherit z-10">
                  <div>
                    <div className="font-semibold">
                      {corridor.source_asset} → {corridor.destination_asset}
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400 font-mono">
                      {corridor.id}
                    </div>
                  </div>
                </td>
                {METRICS.map((metric) => {
                  const value = corridor[metric.key] as number;
                  return (
                    <td
                      key={metric.key}
                      className={`px-4 py-4 whitespace-nowrap text-sm border-l border-gray-100 dark:border-slate-700 ${getCellColor(
                        value,
                        metric
                      )}`}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-medium text-gray-900 dark:text-white">
                          {metric.format(value)}
                        </span>
                        <div className="flex items-center gap-1">
                          {getTrendIndicator(value, metric)}
                          {getIndicator(value, metric)}
                        </div>
                      </div>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Legend */}
      <div className="px-4 py-3 bg-gray-50 dark:bg-slate-900 border-t border-gray-200 dark:border-slate-700">
        <div className="flex flex-wrap items-center gap-4 text-xs text-gray-600 dark:text-gray-400">
          <div className="flex items-center gap-1">
            <Trophy className="w-3 h-3 text-yellow-500" />
            <span>Best Performance</span>
          </div>
          <div className="flex items-center gap-1">
            <AlertTriangle className="w-3 h-3 text-red-500" />
            <span>Worst Performance</span>
          </div>
          <div className="flex items-center gap-1">
            <TrendingUp className="w-3 h-3 text-green-500" />
            <span>Above Average</span>
          </div>
          <div className="flex items-center gap-1">
            <TrendingDown className="w-3 h-3 text-red-500" />
            <span>Below Average</span>
          </div>
        </div>
      </div>
    </div>
  );
}