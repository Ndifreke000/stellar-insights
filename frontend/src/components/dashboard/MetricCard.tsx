import React from 'react';
import { ArrowUpRight, ArrowDownRight } from 'lucide-react';

interface MetricCardProps {
  label: string;
  value: string | number;
  trend?: number;
  trendDirection?: 'up' | 'down';
  subLabel?: string;
}

export const MetricCard: React.FC<MetricCardProps> = ({ label, value, trend, trendDirection, subLabel }) => {
  return (
    <div className="bg-card text-card-foreground rounded-xl border p-6 shadow-sm">
      <div className="flex flex-row items-center justify-between space-y-0 pb-2">
        <h3 className="tracking-tight text-sm font-medium">{label}</h3>
      </div>
      <div className="flex flex-col">
        <div className="text-2xl font-bold">{value}</div>
        {trend !== undefined && (
          <div className={`text-xs flex items-center mt-1 ${trendDirection === 'up' ? 'text-green-500' : 'text-red-500'}`}>
            {trendDirection === 'up' ? <ArrowUpRight className="h-4 w-4 mr-1" /> : <ArrowDownRight className="h-4 w-4 mr-1" />}
            <span className="font-medium">{Math.abs(trend)}%</span>
            <span className="text-muted-foreground ml-1">vs last month</span>
          </div>
        )}
        {subLabel && <p className="text-xs text-muted-foreground mt-1">{subLabel}</p>}
      </div>
    </div>
  );
};
