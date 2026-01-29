"use client";

import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Tooltip,
  TooltipProps,
} from "recharts";
import { IssuedAsset } from "@/lib/api";

interface AssetDistributionChartProps {
  assets: IssuedAsset[];
}

const COLORS = [
  "#6366f1", // Indigo 500
  "#ec4899", // Pink 500
  "#10b981", // Emerald 500
  "#f59e0b", // Amber 500
  "#8b5cf6", // Violet 500
  "#3b82f6", // Blue 500
  "#ef4444", // Red 500
  "#14b8a6", // Teal 500
];

const CustomTooltip = ({ active, payload }: TooltipProps<number, string>) => {
  if (active && payload && payload.length) {
    const data = payload[0].payload;
    return (
      <div className="bg-slate-900 border border-slate-700 p-3 rounded-lg shadow-xl">
        <div className="flex items-center gap-2 mb-1">
          <div
            className="w-3 h-3 rounded-full"
            style={{ backgroundColor: data.fill }}
          ></div>
          <span className="text-white font-medium">{data.asset_code}</span>
        </div>
        <div className="text-slate-400 text-xs">
          Volume (24h):{" "}
          <span className="text-white font-mono">
            {new Intl.NumberFormat("en-US", {
              style: "currency",
              currency: "USD",
              maximumFractionDigits: 0,
            }).format(data.volume_24h_usd)}
          </span>
        </div>
        <div className="text-slate-400 text-xs">
          Share:{" "}
          <span className="text-white font-mono">
            {(data.percent * 100).toFixed(1)}%
          </span>
        </div>
      </div>
    );
  }
  return null;
};

export function AssetDistributionChart({
  assets,
}: AssetDistributionChartProps) {
  if (assets.length === 0) {
    return (
      <div className="h-[300px] flex items-center justify-center text-slate-500 text-sm">
        No asset data available
      </div>
    );
  }

  const totalVolume = assets.reduce(
    (sum, asset) => sum + asset.volume_24h_usd,
    0,
  );

  const data = assets
    .map((asset, index) => ({
      ...asset,
      value: asset.volume_24h_usd,
      percent: totalVolume > 0 ? asset.volume_24h_usd / totalVolume : 0,
      fill: COLORS[index % COLORS.length],
    }))
    .filter((item) => item.value > 0)
    .sort((a, b) => b.value - a.value);

  return (
    <div className="bg-slate-900 border border-slate-800 rounded-xl p-6 shadow-sm h-full flex flex-col">
      <h3 className="font-semibold text-white mb-2">Asset Distribution</h3>
      <p className="text-sm text-slate-400 mb-6">By 24h Volume</p>

      <div className="flex-1 min-h-[250px] relative">
        <ResponsiveContainer width="100%" height="100%">
          <PieChart>
            <Pie
              data={data}
              innerRadius={60}
              outerRadius={80}
              paddingAngle={5}
              dataKey="value"
              stroke="none"
            >
              {data.map((entry, index) => (
                <Cell key={`cell-${index}`} fill={entry.fill} />
              ))}
            </Pie>
            <Tooltip content={CustomTooltip} />
          </PieChart>
        </ResponsiveContainer>

        {/* Center Text */}
        <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
          <div className="text-center">
            <div className="text-xs text-slate-400 uppercase tracking-wider">
              Total Vol
            </div>
            <div className="text-lg font-bold text-white font-mono">
              {new Intl.NumberFormat("en-US", {
                style: "currency",
                currency: "USD",
                notation: "compact",
                maximumFractionDigits: 1,
              }).format(totalVolume)}
            </div>
          </div>
        </div>
      </div>

      {/* Legend */}
      <div className="mt-6 flex flex-wrap gap-3 justify-center">
        {data.map((entry, index) => (
          <div
            key={entry.asset_code}
            className="flex items-center gap-1.5 text-xs"
          >
            <div
              className="w-2.5 h-2.5 rounded-full"
              style={{ backgroundColor: entry.fill }}
            ></div>
            <span className="text-slate-300">{entry.asset_code}</span>
            <span className="text-slate-500 font-mono">
              {(entry.percent * 100).toFixed(0)}%
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
