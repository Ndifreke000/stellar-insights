// frontend/src/components/charts/MarketDepthChart.tsx
"use client";

import { useMemo } from "react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ReferenceLine,
  ResponsiveContainer,
} from "recharts";

// ─── Types ────────────────────────────────────────────────────────────────────

interface PriceLevel {
  price: number;
  amount: number;
}

interface OrderBook {
  bids: PriceLevel[];
  asks: PriceLevel[];
}

export interface LiquidityMetrics {
  total_bid_volume: number;
  total_ask_volume: number;
  best_bid: number;
  best_ask: number;
  spread: number;
  spread_bps: number;
  mid_price: number;
  depth_at_1_percent: number;
  depth_at_5_percent: number;
  fetched_at: number;
}

interface Props {
  orderBook: OrderBook;
  metrics?: LiquidityMetrics;
  baseCurrency?: string;
  counterCurrency?: string;
}

// ─── Chart data helpers ───────────────────────────────────────────────────────

/** Build cumulative depth curve for bids (descending prices) */
function buildBidCurve(bids: PriceLevel[]): { price: number; bidVolume: number }[] {
  const sorted = [...bids].sort((a, b) => b.price - a.price); // best bid first
  let cumulative = 0;
  return sorted.map((level) => {
    cumulative += level.amount;
    return { price: level.price, bidVolume: cumulative };
  });
}

/** Build cumulative depth curve for asks (ascending prices) */
function buildAskCurve(asks: PriceLevel[]): { price: number; askVolume: number }[] {
  const sorted = [...asks].sort((a, b) => a.price - b.price); // best ask first
  let cumulative = 0;
  return sorted.map((level) => {
    cumulative += level.amount;
    return { price: level.price, askVolume: cumulative };
  });
}

/** Merge bid and ask arrays into one timeline keyed on price. */
function mergeDepthData(
  bids: PriceLevel[],
  asks: PriceLevel[]
): { price: number; bidVolume?: number; askVolume?: number }[] {
  const bidCurve = buildBidCurve(bids);
  const askCurve = buildAskCurve(asks);

  const merged = new Map<number, { bidVolume?: number; askVolume?: number }>();

  for (const { price, bidVolume } of bidCurve) {
    merged.set(price, { ...merged.get(price), bidVolume });
  }
  for (const { price, askVolume } of askCurve) {
    merged.set(price, { ...merged.get(price), askVolume });
  }

  return Array.from(merged.entries())
    .map(([price, vals]) => ({ price, ...vals }))
    .sort((a, b) => a.price - b.price);
}

// ─── Custom tooltip ───────────────────────────────────────────────────────────

const CustomTooltip = ({
  active,
  payload,
  label,
  baseCurrency = "",
}: any) => {
  if (!active || !payload?.length) return null;
  const bidVol = payload.find((p: any) => p.dataKey === "bidVolume")?.value;
  const askVol = payload.find((p: any) => p.dataKey === "askVolume")?.value;
  return (
    <div className="bg-gray-900 border border-gray-700 rounded p-3 text-sm shadow-lg">
      <p className="font-semibold text-white mb-1">Price: {Number(label).toFixed(6)}</p>
      {bidVol != null && (
        <p className="text-emerald-400">
          Bid depth: {Number(bidVol).toLocaleString()} {baseCurrency}
        </p>
      )}
      {askVol != null && (
        <p className="text-red-400">
          Ask depth: {Number(askVol).toLocaleString()} {baseCurrency}
        </p>
      )}
    </div>
  );
};

// ─── Component ────────────────────────────────────────────────────────────────

export default function MarketDepthChart({
  orderBook,
  metrics,
  baseCurrency = "BASE",
  counterCurrency = "COUNTER",
}: Props) {
  const data = useMemo(
    () => mergeDepthData(orderBook.bids, orderBook.asks),
    [orderBook]
  );

  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center h-48 rounded-lg bg-gray-900 text-gray-500 text-sm">
        No order book data available
      </div>
    );
  }

  const midPrice = metrics?.mid_price;

  return (
    <div className="bg-gray-900 rounded-xl p-4 space-y-4">
      {/* Header metrics */}
      {metrics && (
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
          <MetricCard label="Mid Price" value={midPrice?.toFixed(6) ?? "—"} />
          <MetricCard
            label="Spread"
            value={`${metrics.spread_bps.toFixed(1)} bps`}
            sub={metrics.spread.toFixed(6)}
          />
          <MetricCard
            label="1% Depth"
            value={`${metrics.depth_at_1_percent.toLocaleString()} ${baseCurrency}`}
          />
          <MetricCard
            label="5% Depth"
            value={`${metrics.depth_at_5_percent.toLocaleString()} ${baseCurrency}`}
          />
        </div>
      )}

      {/* Chart */}
      <div className="h-64">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data} margin={{ top: 4, right: 16, left: 8, bottom: 0 }}>
            <defs>
              <linearGradient id="bidGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#10b981" stopOpacity={0.4} />
                <stop offset="95%" stopColor="#10b981" stopOpacity={0.05} />
              </linearGradient>
              <linearGradient id="askGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#ef4444" stopOpacity={0.4} />
                <stop offset="95%" stopColor="#ef4444" stopOpacity={0.05} />
              </linearGradient>
            </defs>

            <CartesianGrid strokeDasharray="3 3" stroke="#374151" />

            <XAxis
              dataKey="price"
              tickFormatter={(v) => Number(v).toFixed(4)}
              tick={{ fill: "#9ca3af", fontSize: 11 }}
              label={{
                value: `Price (${counterCurrency})`,
                position: "insideBottom",
                offset: -2,
                fill: "#6b7280",
                fontSize: 11,
              }}
            />

            <YAxis
              tickFormatter={(v) =>
                v >= 1_000 ? `${(v / 1000).toFixed(0)}k` : String(v)
              }
              tick={{ fill: "#9ca3af", fontSize: 11 }}
              label={{
                value: `Volume (${baseCurrency})`,
                angle: -90,
                position: "insideLeft",
                fill: "#6b7280",
                fontSize: 11,
              }}
            />

            <Tooltip
              content={<CustomTooltip baseCurrency={baseCurrency} />}
              cursor={{ stroke: "#6b7280", strokeWidth: 1, strokeDasharray: "4 2" }}
            />

            {midPrice != null && (
              <ReferenceLine
                x={midPrice}
                stroke="#f59e0b"
                strokeWidth={1.5}
                strokeDasharray="6 3"
                label={{
                  value: "Mid",
                  position: "top",
                  fill: "#f59e0b",
                  fontSize: 11,
                }}
              />
            )}

            <Area
              type="stepAfter"
              dataKey="bidVolume"
              name="Bid Depth"
              stroke="#10b981"
              strokeWidth={1.5}
              fill="url(#bidGradient)"
              connectNulls={false}
              dot={false}
              activeDot={{ r: 3, stroke: "#10b981" }}
            />

            <Area
              type="stepBefore"
              dataKey="askVolume"
              name="Ask Depth"
              stroke="#ef4444"
              strokeWidth={1.5}
              fill="url(#askGradient)"
              connectNulls={false}
              dot={false}
              activeDot={{ r: 3, stroke: "#ef4444" }}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      {/* Volume summary */}
      {metrics && (
        <div className="flex justify-between text-xs text-gray-500">
          <span className="text-emerald-400">
            ▲ Total Bids: {metrics.total_bid_volume.toLocaleString()} {baseCurrency}
          </span>
          <span className="text-gray-400">
            Updated {new Date(metrics.fetched_at * 1000).toLocaleTimeString()}
          </span>
          <span className="text-red-400">
            ▼ Total Asks: {metrics.total_ask_volume.toLocaleString()} {baseCurrency}
          </span>
        </div>
      )}
    </div>
  );
}

// ─── Small helper card ────────────────────────────────────────────────────────

function MetricCard({
  label,
  value,
  sub,
}: {
  label: string;
  value: string;
  sub?: string;
}) {
  return (
    <div className="bg-gray-800 rounded-lg px-3 py-2">
      <p className="text-xs text-gray-400">{label}</p>
      <p className="text-sm font-semibold text-white truncate">{value}</p>
      {sub && <p className="text-xs text-gray-500">{sub}</p>}
    </div>
  );
}
