'use client';

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import { TVLDataPoint } from '@/lib/analytics-api';

interface TVLChartProps {
  data: TVLDataPoint[];
}

export function TVLChart({ data }: TVLChartProps) {
  const chartData = data.map((point) => ({
    timestamp: new Date(point.timestamp).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    }),
    tvl_usd: Math.round(point.tvl_usd),
  }));

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      notation: 'compact',
      maximumFractionDigits: 1,
    }).format(value);
  };

  const maxTVL = Math.max(...chartData.map((d) => d.tvl_usd));
  const minTVL = Math.min(...chartData.map((d) => d.tvl_usd));
  const avgTVL = chartData.reduce((sum, d) => sum + d.tvl_usd, 0) / chartData.length;

  return (
    <div className="glass-card rounded-2xl p-6 border border-border/50">
      <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">Network Capital // 03.B</div>
      <h2 className="text-xl font-black tracking-tighter uppercase italic mb-2">
        Total Value Locked
      </h2>
      <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mb-6">
        Aggregated TVL across verified anchors
      </p>

      {/* Summary Stats */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-wider mb-1">
            Current
          </p>
          <p className="text-xl font-black font-mono tracking-tighter text-emerald-400">
            {formatCurrency(chartData[chartData.length - 1]?.tvl_usd || 0)}
          </p>
        </div>
        <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-wider mb-1">
            Average
          </p>
          <p className="text-xl font-black font-mono tracking-tighter text-foreground/80">
            {formatCurrency(avgTVL)}
          </p>
        </div>
        <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-wider mb-1">
            Volatility
          </p>
          <p className="text-xl font-black font-mono tracking-tighter text-accent">
            {formatCurrency(maxTVL - minTVL)}
          </p>
        </div>
      </div>

      <div className="h-[350px] w-full">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={chartData}>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" vertical={false} />
            <XAxis
              dataKey="timestamp"
              stroke="rgba(255,255,255,0.3)"
              tick={{ fontSize: 10, fontFamily: 'monospace' }}
              axisLine={false}
              tickLine={false}
              dy={10}
            />
            <YAxis
              stroke="rgba(255,255,255,0.3)"
              tickFormatter={formatCurrency}
              tick={{ fontSize: 10, fontFamily: 'monospace' }}
              axisLine={false}
              tickLine={false}
              dx={-10}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: 'rgba(15, 23, 42, 0.9)',
                border: '1px solid rgba(255, 255, 255, 0.1)',
                borderRadius: '12px',
                backdropFilter: 'blur(12px)',
                fontSize: '10px',
                fontFamily: 'monospace'
              }}
              labelStyle={{ color: '#94a3b8', marginBottom: '4px' }}
            />
            <Line
              type="monotone"
              dataKey="tvl_usd"
              stroke="#10b981"
              strokeWidth={3}
              dot={false}
              activeDot={{ r: 4, fill: '#10b981', stroke: '#fff', strokeWidth: 2 }}
              name="TVL"
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
