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
import { LiquidityDataPoint } from '@/lib/analytics-api';

interface LiquidityChartProps {
  data: LiquidityDataPoint[];
}

export function LiquidityChart({ data }: LiquidityChartProps) {
  // Group data by corridor for better visualization
  const corridorData = new Map<
    string,
    { timestamp: string; liquidity_usd: number }[]
  >();

  data.forEach((point) => {
    if (!corridorData.has(point.corridor_key)) {
      corridorData.set(point.corridor_key, []);
    }
    corridorData.get(point.corridor_key)!.push({
      timestamp: point.timestamp,
      liquidity_usd: point.liquidity_usd,
    });
  });

  // Use aggregated data or show just top corridor for clarity
  const aggregatedData = new Map<string, number>();
  data.forEach((point) => {
    const timestamp = point.timestamp.split('T')[0]; // Date only
    const current = aggregatedData.get(timestamp) || 0;
    aggregatedData.set(timestamp, current + point.liquidity_usd);
  });

  const chartData = Array.from(aggregatedData.entries())
    .map(([timestamp, liquidity]) => ({
      timestamp: new Date(timestamp).toLocaleDateString('en-US', {
        month: 'short',
        day: 'numeric',
      }),
      liquidity_usd: Math.round(liquidity),
    }))
    .sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      notation: 'compact',
      maximumFractionDigits: 1,
    }).format(value);
  };

  return (
    <div className="glass-card rounded-2xl p-6 border border-border/50">
      <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">Market Velocity // 05.A</div>
      <h2 className="text-xl font-black tracking-tighter uppercase italic mb-2">
        Liquidity Over Time
      </h2>
      <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider mb-8">
        Global Depth Index across verified corridors
      </p>

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
                fontFamily: 'monospace',
                textTransform: 'uppercase'
              }}
              itemStyle={{ color: '#6366f1', fontWeight: 'bold' }}
              labelStyle={{ color: '#94a3b8', marginBottom: '4px' }}
              formatter={(value: number) => [formatCurrency(value), 'GLOBAL_DEPTH']}
            />
            <Line
              type="monotone"
              dataKey="liquidity_usd"
              stroke="#6366f1"
              strokeWidth={3}
              dot={false}
              activeDot={{ r: 4, fill: '#6366f1', stroke: '#fff', strokeWidth: 2 }}
              name="Liquidity"
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
