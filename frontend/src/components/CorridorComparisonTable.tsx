import React, { useState } from 'react';
import { 
  TrendingUp, 
  TrendingDown, 
  Minus, 
  Download,
  Trophy,
  AlertTriangle,
  ChevronLeft,
  ChevronRight
} from 'lucide-react';
import { CorridorMetrics } from '@/lib/api';

// --- Pagination Component ---
interface PaginationProps {
    totalItems: number;
    pageSize: number;
    currentPage: number;
    onPageChange: (page: number) => void;
    onPageSizeChange: (size: number) => void;
}

export function DataTablePagination({
    totalItems,
    pageSize,
    currentPage,
    onPageChange,
    onPageSizeChange,
}: PaginationProps) {
    const totalPages = Math.max(1, Math.ceil(totalItems / pageSize));
    const pageSizes = [10, 25, 50, 100];

    const handleJumpToPage = (e: React.ChangeEvent<HTMLInputElement>) => {
        const value = parseInt(e.target.value);
        if (!isNaN(value)) {
            if (value < 1) onPageChange(1);
            else if (value > totalPages) onPageChange(totalPages);
            else onPageChange(value);
        }
    };

    return (
        <nav 
            className="flex flex-col sm:flex-row items-center justify-between gap-4 px-4 py-4 bg-slate-900/50 border-t border-slate-800"
            aria-label="Table pagination"
        >
            <div className="text-sm text-slate-400" role="status" aria-live="polite">
                Total <span className="font-medium text-white">{totalItems}</span> records
            </div>

            <div className="flex flex-wrap items-center gap-4 sm:gap-6">
                <div className="flex items-center gap-2">
                    <label htmlFor="pageSize" className="text-sm text-slate-400">Rows per page</label>
                    <select
                        id="pageSize"
                        value={pageSize}
                        onChange={(e) => onPageSizeChange(Number(e.target.value))}
                        className="bg-slate-800 border border-slate-700 text-white text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block p-1.5"
                    >
                        {pageSizes.map((size) => (
                            <option key={size} value={size}>
                                {size}
                            </option>
                        ))}
                    </select>
                </div>

                <div className="flex items-center gap-2">
                    <label htmlFor="jumpToPage" className="text-sm text-slate-400">Jump to</label>
                    <input
                        id="jumpToPage"
                        type="number"
                        min={1}
                        max={totalPages}
                        value={currentPage}
                        onChange={handleJumpToPage}
                        aria-label={`Current page, ${currentPage} of ${totalPages}`}
                        className="w-16 bg-slate-800 border border-slate-700 text-white text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block p-1.5"
                    />
                    <span className="text-sm text-slate-400" aria-hidden="true">of {totalPages}</span>
                </div>

                <div className="flex items-center gap-1">
                    <button
                        onClick={() => onPageChange(1)}
                        disabled={currentPage === 1}
                        aria-label="First Page"
                        className="p-1.5 rounded-md hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed text-slate-400 hover:text-white transition-colors"
                    >
                        <ChevronLeft className="w-5 h-5" aria-hidden="true" />
                    </button>
                    <button
                        onClick={() => onPageChange(currentPage - 1)}
                        disabled={currentPage === 1}
                        aria-label="Previous Page"
                        className="p-1.5 rounded-md hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed text-slate-400 hover:text-white transition-colors"
                    >
                        <ChevronLeft className="w-5 h-5" aria-hidden="true" />
                    </button>

                    <div 
                        className="px-3 py-1.5 text-sm font-medium text-white bg-blue-600 rounded-md"
                        aria-current="page"
                        aria-label={`Page ${currentPage}`}
                    >
                        {currentPage}
                    </div>

                    <button
                        onClick={() => onPageChange(currentPage + 1)}
                        disabled={currentPage === totalPages}
                        aria-label="Next Page"
                        className="p-1.5 rounded-md hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed text-slate-400 hover:text-white transition-colors"
                    >
                        <ChevronRight className="w-5 h-5" aria-hidden="true" />
                    </button>
                    <button
                        onClick={() => onPageChange(totalPages)}
                        disabled={currentPage === totalPages}
                        aria-label="Last Page"
                        className="p-1.5 rounded-md hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed text-slate-400 hover:text-white transition-colors"
                    >
                        <ChevronRight className="w-5 h-5" aria-hidden="true" />
                    </button>
                </div>
            </div>
        </nav>
    );
}

// --- Table Component ---
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
  { label: 'Success Rate', key: 'success_rate', format: (v) => `${v.toFixed(2)}%`, higherIsBetter: true },
  { label: 'Health Score', key: 'health_score', format: (v) => v.toFixed(1), higherIsBetter: true },
  { label: 'Avg Latency', key: 'average_latency_ms', format: (v) => `${v.toFixed(0)}ms`, higherIsBetter: false },
  { label: 'Liquidity Depth', key: 'liquidity_depth_usd', format: (v) => `$${(v / 1000000).toFixed(2)}M`, higherIsBetter: true },
  { label: '24h Volume', key: 'liquidity_volume_24h_usd', format: (v) => `$${(v / 1000).toFixed(0)}K`, higherIsBetter: true },
  { label: 'Avg Slippage', key: 'average_slippage_bps', format: (v) => `${v.toFixed(2)} bps`, higherIsBetter: false },
];

export function CorridorComparisonTable({ corridors, onExport }: CorridorComparisonTableProps) {
  const [sortBy, setSortBy] = useState<MetricKey>('success_rate');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');

  const getBestWorst = (metric: MetricConfig) => {
    const values = corridors.map((c) => c[metric.key] as number);
    const best = metric.higherIsBetter ? Math.max(...values) : Math.min(...values);
    const worst = metric.higherIsBetter ? Math.min(...values) : Math.max(...values);
    return { best, worst };
  };

  const getIndicator = (value: number, metric: MetricConfig) => {
    const { best, worst } = getBestWorst(metric);
    if (value === best) return <Trophy className="w-4 h-4 text-yellow-500" aria-label="Best performance in category" />;
    if (value === worst) return <AlertTriangle className="w-4 h-4 text-red-500" aria-label="Worst performance in category" />;
    return null;
  };

  const getTrendIndicator = (value: number, metric: MetricConfig) => {
    const { best, worst } = getBestWorst(metric);
    const range = Math.abs(best - worst);
    const position = Math.abs(value - worst) / range;
    if (position > 0.8) return <TrendingUp className="w-4 h-4 text-green-500" aria-label="Trending high" />;
    if (position < 0.3) return <TrendingDown className="w-4 h-4 text-red-500" aria-label="Trending low" />;
    return <Minus className="w-4 h-4 text-muted-foreground" aria-label="Stable" />;
  };

  const getCellColor = (value: number, metric: MetricConfig) => {
    const { best, worst } = getBestWorst(metric);
    if (value === best) return 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-700';
    if (value === worst) return 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-700';
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
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-slate-700">
        <h3 id="table-caption" className="text-lg font-semibold text-gray-900 dark:text-white">
          Detailed Comparison
        </h3>
        {onExport && (
          <button
            onClick={onExport}
            aria-label="Export comparison data to CSV"
            className="flex items-center gap-2 px-3 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          >
            <Download className="w-4 h-4" aria-hidden="true" />
            Export CSV
          </button>
        )}
      </div>

      <div className="overflow-x-auto">
        <table className="w-full" aria-labelledby="table-caption">
          <thead className="bg-gray-50 dark:bg-slate-900">
            <tr>
              <th 
                scope="col"
                className="px-4 py-3 text-left text-xs font-medium text-muted-foreground dark:text-muted-foreground uppercase tracking-wider sticky left-0 bg-gray-50 dark:bg-slate-900 z-10"
              >
                Corridor
              </th>
              {METRICS.map((metric) => (
                <th
                  key={metric.key}
                  scope="col"
                  aria-sort={sortBy === metric.key ? (sortOrder === 'asc' ? 'ascending' : 'descending') : 'none'}
                  className="px-4 py-3 text-left text-xs font-medium text-muted-foreground dark:text-muted-foreground uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-800 transition-colors"
                  onClick={() => handleSort(metric.key)}
                >
                  <button 
                    className="flex items-center gap-2 w-full uppercase"
                    aria-label={`Sort by ${metric.label}`}
                  >
                    {metric.label}
                    {sortBy === metric.key && (
                      sortOrder === 'asc' ? 
                        <TrendingUp className="w-3 h-3" aria-hidden="true" /> : 
                        <TrendingDown className="w-3 h-3" aria-hidden="true" />
                    )}
                  </button>
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
                <th 
                  scope="row"
                  className="px-4 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white sticky left-0 bg-inherit z-10"
                >
                  <div>
                    <div className="font-semibold">
                      {corridor.source_asset} → {corridor.destination_asset}
                    </div>
                    <div className="text-xs text-muted-foreground dark:text-muted-foreground font-mono">
                      {corridor.id}
                    </div>
                  </div>
                </th>
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
                          <span className="sr-only">{metric.label}: </span>
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

      <div className="px-4 py-3 bg-gray-50 dark:bg-slate-900 border-t border-gray-200 dark:border-slate-700" aria-label="Table Legend">
        <div className="flex flex-wrap items-center gap-4 text-xs text-muted-foreground dark:text-muted-foreground">
          <div className="flex items-center gap-1">
            <Trophy className="w-3 h-3 text-yellow-500" aria-hidden="true" />
            <span>Best Performance</span>
          </div>
          <div className="flex items-center gap-1">
            <AlertTriangle className="w-3 h-3 text-red-500" aria-hidden="true" />
            <span>Worst Performance</span>
          </div>
          <div className="flex items-center gap-1">
            <TrendingUp className="w-3 h-3 text-green-500" aria-hidden="true" />
            <span>Above Average</span>
          </div>
          <div className="flex items-center gap-1">
            <TrendingDown className="w-3 h-3 text-red-500" aria-hidden="true" />
            <span>Below Average</span>
          </div>
        </div>
      </div>
    </div>
  );
}