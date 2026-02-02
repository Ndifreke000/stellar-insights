'use client';

import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import { SettlementLatencyDataPoint } from '@/lib/analytics-api';

interface SettlementLatencyChartProps {
  data: SettlementLatencyDataPoint[];
}

export function SettlementLatencyChart({ data }: SettlementLatencyChartProps) {
  const chartData = data.map((point) => ({
    timestamp: new Date(point.timestamp).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
    }),
    median: Math.round(point.median_latency_ms),
    p95: Math.round(point.p95_latency_ms),
    p99: Math.round(point.p99_latency_ms),
  }));

  const formatLatency = (value: number) => {
    return `${value}ms`;
  };

  const latestPoint = data[data.length - 1];

  return (
    <div className="glass-card rounded-2xl p-6 border border-border/50">
      <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">Network Timing // 03.C</div>
      <h2 className="text-xl font-black tracking-tighter uppercase italic mb-2">
        Settlement Latency
      </h2>
      <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mb-6">
        Median and percentile settlement times
      </p>

      {/* Summary Stats */}
      <div className="grid grid-cols-3 gap-4 mb-8">
        <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-wider mb-1">
            Median
          </p>
          <p className="text-xl font-black font-mono tracking-tighter text-foreground">
            {formatLatency(Math.round(latestPoint?.median_latency_ms || 0))}
          </p>
        </div>
        <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-wider mb-1">
            P95
          </p>
          <p className="text-xl font-black font-mono tracking-tighter text-amber-400">
            {formatLatency(Math.round(latestPoint?.p95_latency_ms || 0))}
          </p>
        </div>
        <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5">
          <p className="text-[9px] font-mono text-muted-foreground uppercase tracking-wider mb-1">
            P99
          </p>
          <p className="text-xl font-black font-mono tracking-tighter text-red-400">
            {formatLatency(Math.round(latestPoint?.p99_latency_ms || 0))}
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
              tickFormatter={formatLatency}
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
              labelStyle={{ color: '#94a3b8', marginBottom: '4px' }}
            />
            <Legend
              verticalAlign="top"
              align="right"
              wrapperStyle={{ fontSize: '9px', fontFamily: 'monospace', paddingBottom: '20px', textTransform: 'uppercase' }}
            />
            <Line
              type="monotone"
              dataKey="median"
              stroke="#6366f1"
              strokeWidth={2}
              dot={false}
              name="Median"
            />
            <Line
              type="monotone"
              dataKey="p95"
              stroke="#f59e0b"
              strokeWidth={2}
              dot={false}
              name="P95"
              strokeDasharray="5 5"
            />
            <Line
              type="monotone"
              dataKey="p99"
              stroke="#ef4444"
              strokeWidth={2}
              dot={false}
              name="P99"
              strokeDasharray="5 5"
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
