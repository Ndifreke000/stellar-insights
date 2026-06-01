import React from 'react';
import { ArrowUpRight, ArrowDownRight, Activity } from 'lucide-react';

interface MetricCardProps {
  label: string;
  value: string | number;
  trend?: number;
  trendDirection?: 'up' | 'down';
  subLabel?: string;
  inverse?: boolean; // If true, "down" is good (e.g., latency)
}

export const MetricCard: React.FC<MetricCardProps> = ({
  label,
  value,
  trend,
  trendDirection,
  subLabel,
  inverse = false
}) => {
  const isPositive = inverse
    ? trendDirection === 'down'
    : trendDirection === 'up';

  const trendColor = isPositive ? 'text-green-400' : 'text-red-400';
  const GlowClass = isPositive ? 'glow-success' : '';

  return (
    <div className="glass-card rounded-2xl p-6 border border-border/50 group hover:border-accent/30 transition-all duration-300">
      <div className="flex flex-row items-center justify-between pb-4">
        <h3 className="text-xs font-bold uppercase tracking-widest text-muted-foreground group-hover:text-accent transition-colors">
          {label}
        </h3>
        <Activity className={`h-4 w-4 text-muted-foreground/30 group-hover:text-accent transition-colors ${GlowClass}`} />
      </div>
      <div className="flex flex-col">
        <div className="text-3xl font-mono font-bold tracking-tighter text-foreground">
          {value}
        </div>

        {trend !== undefined && (
          <div className={`text-[10px] font-mono flex items-center mt-2 ${trendColor}`}>
            {trendDirection === 'up' ? <ArrowUpRight className="h-3 w-3 mr-1" /> : <ArrowDownRight className="h-3 w-3 mr-1" />}
            <span className="font-bold">{Math.abs(trend)}%</span>
            <span className="text-muted-foreground/50 ml-2 uppercase tracking-tighter">vs prev window</span>
          </div>
        )}

        {subLabel && <p className="text-[10px] font-mono text-muted-foreground/50 mt-2 uppercase tracking-tighter">{subLabel}</p>}
      </div>
    </div>
  );
};
