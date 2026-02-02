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
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      notation: 'compact',
      maximumFractionDigits: 1,
    }).format(value);
  };

  const getCorridorLabel = (corridor: CorridorAnalytics) => {
    return `${corridor.asset_a_code} → ${corridor.asset_b_code}`;
  };

  return (
    <div className="glass-card rounded-2xl p-6 border border-border/50">
      <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">Network Performance // 03.D</div>
      <h2 className="text-xl font-black tracking-tighter uppercase italic mb-2">
        Top Corridors
      </h2>
      <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mb-6">
        Highest performing payment channels
      </p>

      <div className="space-y-3">
        {sortedCorridors.slice(0, 5).map((corridor, index) => (
          <div
            key={corridor.corridor_key}
            className="flex items-center justify-between p-4 bg-slate-900/40 border border-white/5 rounded-xl hover:border-accent/30 transition-all group"
          >
            <div className="flex items-center gap-4 flex-1">
              <div className="flex items-center justify-center w-8 h-8 bg-accent/10 border border-accent/20 rounded-lg text-xs font-mono font-black text-accent group-hover:bg-accent group-hover:text-white transition-all">
                {String(index + 1).padStart(2, '0')}
              </div>
              <div className="flex-1">
                <p className="font-bold tracking-tight text-foreground text-sm uppercase">
                  {getCorridorLabel(corridor)}
                </p>
                <div className="flex gap-4 mt-1 text-[9px] font-mono text-muted-foreground uppercase tracking-wider">
                  <span>Vol: <span className="text-foreground/70">{formatCurrency(corridor.volume_usd)}</span></span>
                  <span>Txns: <span className="text-foreground/70">{corridor.total_transactions}</span></span>
                  {corridor.avg_settlement_latency_ms && (
                    <span>Lat: <span className="text-foreground/70">{corridor.avg_settlement_latency_ms}ms</span></span>
                  )}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-6">
              <div className="text-right">
                <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-widest mb-1">
                  Depth
                </p>
                <p className="text-xs font-black font-mono text-foreground uppercase tracking-tighter">
                  {formatCurrency(corridor.liquidity_depth_usd)}
                </p>
              </div>

              <div className="text-right">
                <span
                  className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-lg text-[10px] font-black font-mono border ${corridor.success_rate >= 98 ? 'bg-green-500/10 border-green-500/30 text-green-400' :
                    corridor.success_rate >= 95 ? 'bg-yellow-500/10 border-yellow-500/30 text-yellow-400' : 'bg-red-500/10 border-red-500/30 text-red-400'
                    }`}
                >
                  <TrendingUp className="w-3 h-3" />
                  {corridor.success_rate.toFixed(1)}%
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Stats Summary Table-like Grid */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 mt-8 pt-6 border-t border-border/20">
        <div className="space-y-1">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-[0.2em]">Total_Vol</p>
          <p className="text-sm font-black font-mono tracking-tighter text-foreground italic">
            {formatCurrency(sortedCorridors.reduce((sum, c) => sum + c.volume_usd, 0))}
          </p>
        </div>
        <div className="space-y-1">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-[0.2em]">Avg_P_Success</p>
          <p className="text-sm font-black font-mono tracking-tighter text-accent italic">
            {(sortedCorridors.reduce((sum, c) => sum + c.success_rate, 0) / sortedCorridors.length).toFixed(1)}%
          </p>
        </div>
        <div className="space-y-1">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-[0.2em]">Total_Tx_Count</p>
          <p className="text-sm font-black font-mono tracking-tighter text-foreground italic">
            {sortedCorridors.reduce((sum, c) => sum + c.total_transactions, 0).toLocaleString()}
          </p>
        </div>
        <div className="space-y-1">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-[0.2em]">Total_Depth</p>
          <p className="text-sm font-black font-mono tracking-tighter text-foreground italic">
            {formatCurrency(sortedCorridors.reduce((sum, c) => sum + c.liquidity_depth_usd, 0))}
          </p>
        </div>
      </div>
    </div>
  );
}
