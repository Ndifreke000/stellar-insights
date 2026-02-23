"use client";

import React, { useState, useEffect } from "react";
import {
  Waves,
  Droplets,
  TrendingUp,
  BarChart3,
  ArrowUpRight,
  ArrowDownRight,
  Percent,
  DollarSign,
  Activity,
  ChevronDown,
} from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { PoolPerformanceChart } from "@/components/charts/PoolPerformanceChart";
import { Badge } from "@/components/ui/badge";
import {
  LiquidityPool,
  PoolSnapshot,
  PoolStats,
  fetchPools,
  fetchPoolStats,
  fetchPoolSnapshots,
} from "@/lib/liquidity-pool-api";

type SortKey =
  | "apy"
  | "volume_24h_usd"
  | "fees_earned_24h_usd"
  | "total_value_usd"
  | "impermanent_loss_pct";
type ChartMetric = "apy" | "volume" | "fees" | "tvl";

export default function LiquidityPoolsPage() {
  const [loading, setLoading] = useState(true);
  const [pools, setPools] = useState<LiquidityPool[]>([]);
  const [stats, setStats] = useState<PoolStats | null>(null);
  const [selectedPool, setSelectedPool] = useState<LiquidityPool | null>(null);
  const [snapshots, setSnapshots] = useState<PoolSnapshot[]>([]);
  const [sortKey, setSortKey] = useState<SortKey>("apy");
  const [sortAsc, setSortAsc] = useState(false);
  const [chartMetric, setChartMetric] = useState<ChartMetric>("apy");

  useEffect(() => {
    async function load() {
      const [poolsData, statsData] = await Promise.all([
        fetchPools(),
        fetchPoolStats(),
      ]);
      setPools(poolsData);
      setStats(statsData);

      if (poolsData.length > 0) {
        setSelectedPool(poolsData[0]);
        const snaps = await fetchPoolSnapshots(poolsData[0].pool_id);
        setSnapshots(snaps);
      }
      setLoading(false);
    }
    load();
  }, []);

  const handleSelectPool = async (pool: LiquidityPool) => {
    setSelectedPool(pool);
    const snaps = await fetchPoolSnapshots(pool.pool_id);
    setSnapshots(snaps);
  };

  const handleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortAsc(!sortAsc);
    } else {
      setSortKey(key);
      setSortAsc(false);
    }
  };

  const sortedPools = [...pools].sort((a, b) => {
    const aVal = a[sortKey];
    const bVal = b[sortKey];
    return sortAsc
      ? (aVal as number) - (bVal as number)
      : (bVal as number) - (aVal as number);
  });

  const formatCurrency = (value: number) =>
    new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(value);

  const formatPercent = (value: number) => `${value.toFixed(2)}%`;

  const getILSeverity = (il: number) => {
    if (il < 1) return "text-emerald-400";
    if (il < 5) return "text-amber-400";
    return "text-red-400";
  };

  if (loading) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest italic">
          Scanning Liquidity Pools... // 404-LP
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      {/* Page Header */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            DeFi Analytics // 06
          </div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <Droplets className="w-8 h-8 text-accent" />
            Liquidity Pools
          </h2>
          <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mt-2">
            Real-time pool performance, APY tracking, and impermanent loss
            analysis
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-border/50 px-3 py-1 bg-accent/5"
          >
            {pools.length} ACTIVE_POOLS
          </Badge>
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-emerald-500/30 px-3 py-1 bg-emerald-500/5 text-emerald-400"
          >
            LIVE_FEED
          </Badge>
        </div>
      </div>

      {/* Overview Metrics */}
      {stats && (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <MetricCard
            label="Total Value Locked"
            value={formatCurrency(stats.total_value_locked_usd)}
            subLabel="All Pools Combined"
            trend={8.2}
            trendDirection="up"
          />
          <MetricCard
            label="24H Volume"
            value={formatCurrency(stats.total_volume_24h_usd)}
            subLabel="Trading Activity"
            trend={12.5}
            trendDirection="up"
          />
          <MetricCard
            label="Average APY"
            value={formatPercent(stats.avg_apy)}
            subLabel="Mean Pool Yield"
            trend={2.3}
            trendDirection="up"
          />
          <MetricCard
            label="24H Fees Earned"
            value={formatCurrency(stats.total_fees_24h_usd)}
            subLabel="Liquidity Provider Revenue"
            trend={5.1}
            trendDirection="up"
          />
        </div>
      )}

      {/* Pool Rankings Table */}
      <div className="glass-card rounded-2xl p-1 border border-border/50">
        <div className="p-6 pb-4">
          <h3 className="text-xs font-mono text-muted-foreground uppercase tracking-widest mb-1 flex items-center gap-2">
            <BarChart3 className="w-3 h-3 text-accent" />
            Pool Rankings
          </h3>
          <p className="text-[10px] font-mono text-muted-foreground/50 uppercase tracking-wider">
            Click a pool to view its performance chart below
          </p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-xs font-mono">
            <thead>
              <tr className="border-b border-border/30">
                <th className="text-left px-6 py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                  Pool
                </th>
                <SortableHeader
                  label="TVL"
                  sortKey="total_value_usd"
                  currentKey={sortKey}
                  ascending={sortAsc}
                  onClick={handleSort}
                />
                <SortableHeader
                  label="APY"
                  sortKey="apy"
                  currentKey={sortKey}
                  ascending={sortAsc}
                  onClick={handleSort}
                />
                <SortableHeader
                  label="24H Vol"
                  sortKey="volume_24h_usd"
                  currentKey={sortKey}
                  ascending={sortAsc}
                  onClick={handleSort}
                />
                <SortableHeader
                  label="24H Fees"
                  sortKey="fees_earned_24h_usd"
                  currentKey={sortKey}
                  ascending={sortAsc}
                  onClick={handleSort}
                />
                <SortableHeader
                  label="IL"
                  sortKey="impermanent_loss_pct"
                  currentKey={sortKey}
                  ascending={sortAsc}
                  onClick={handleSort}
                />
                <th className="text-right px-6 py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                  Trades
                </th>
              </tr>
            </thead>
            <tbody>
              {sortedPools.map((pool) => (
                <tr
                  key={pool.pool_id}
                  onClick={() => handleSelectPool(pool)}
                  className={`border-b border-border/10 cursor-pointer transition-all duration-200 hover:bg-accent/5 ${
                    selectedPool?.pool_id === pool.pool_id
                      ? "bg-accent/10 border-accent/20"
                      : ""
                  }`}
                >
                  <td className="px-6 py-4">
                    <div className="flex items-center gap-2">
                      <div className="w-7 h-7 rounded-full bg-gradient-to-br from-accent/30 to-accent/10 flex items-center justify-center text-[9px] font-black text-accent">
                        {pool.reserve_a_asset_code.charAt(0)}
                      </div>
                      <div>
                        <div className="font-bold text-foreground text-xs">
                          {pool.reserve_a_asset_code}/
                          {pool.reserve_b_asset_code}
                        </div>
                        <div className="text-[9px] text-muted-foreground/50">
                          {pool.fee_bp / 100}% fee
                        </div>
                      </div>
                    </div>
                  </td>
                  <td className="px-6 py-4 text-right font-bold">
                    {formatCurrency(pool.total_value_usd)}
                  </td>
                  <td className="px-6 py-4 text-right">
                    <span className="font-bold text-emerald-400">
                      {formatPercent(pool.apy)}
                    </span>
                  </td>
                  <td className="px-6 py-4 text-right font-bold">
                    {formatCurrency(pool.volume_24h_usd)}
                  </td>
                  <td className="px-6 py-4 text-right font-bold">
                    {formatCurrency(pool.fees_earned_24h_usd)}
                  </td>
                  <td className="px-6 py-4 text-right">
                    <span
                      className={`font-bold ${getILSeverity(pool.impermanent_loss_pct)}`}
                    >
                      {formatPercent(pool.impermanent_loss_pct)}
                    </span>
                  </td>
                  <td className="px-6 py-4 text-right text-muted-foreground">
                    {pool.trade_count_24h.toLocaleString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Performance Chart + Pool Detail */}
      {selectedPool && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-8">
            {/* Chart Metric Selector */}
            <div className="flex items-center gap-2 mb-4">
              {(["apy", "volume", "fees", "tvl"] as ChartMetric[]).map((m) => (
                <button
                  key={m}
                  onClick={() => setChartMetric(m)}
                  className={`px-4 py-2 rounded-xl text-[10px] font-mono font-bold uppercase tracking-widest transition-all duration-200 border ${
                    chartMetric === m
                      ? "bg-accent/20 border-accent/50 text-accent"
                      : "bg-transparent border-border/30 text-muted-foreground hover:border-accent/30 hover:text-foreground"
                  }`}
                >
                  {m}
                </button>
              ))}
            </div>
            <PoolPerformanceChart snapshots={snapshots} metric={chartMetric} />
          </div>

          {/* Pool Detail Panel */}
          <div className="lg:col-span-4 glass-card rounded-2xl p-6 border border-border/50 space-y-6">
            <div>
              <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
                Selected Pool // Detail
              </div>
              <h3 className="text-2xl font-black tracking-tighter uppercase italic">
                {selectedPool.reserve_a_asset_code}/
                {selectedPool.reserve_b_asset_code}
              </h3>
              <p className="text-[9px] font-mono text-muted-foreground/50 mt-1 break-all">
                {selectedPool.pool_id}
              </p>
            </div>

            {/* Pool Composition */}
            <div className="space-y-3">
              <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest">
                Reserves
              </div>
              <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5">
                <div className="flex justify-between items-center">
                  <span className="text-[10px] font-mono text-muted-foreground">
                    {selectedPool.reserve_a_asset_code}
                  </span>
                  <span className="text-sm font-mono font-bold">
                    {selectedPool.reserve_a_amount.toLocaleString()}
                  </span>
                </div>
              </div>
              <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5">
                <div className="flex justify-between items-center">
                  <span className="text-[10px] font-mono text-muted-foreground">
                    {selectedPool.reserve_b_asset_code}
                  </span>
                  <span className="text-sm font-mono font-bold">
                    {selectedPool.reserve_b_amount.toLocaleString()}
                  </span>
                </div>
              </div>
            </div>

            {/* Key Metrics */}
            <div className="grid grid-cols-2 gap-3">
              <DetailStat
                label="APY"
                value={formatPercent(selectedPool.apy)}
                icon={<TrendingUp className="w-3 h-3" />}
                color="text-emerald-400"
              />
              <DetailStat
                label="Imp. Loss"
                value={formatPercent(selectedPool.impermanent_loss_pct)}
                icon={<ArrowDownRight className="w-3 h-3" />}
                color={getILSeverity(selectedPool.impermanent_loss_pct)}
              />
              <DetailStat
                label="Fee Rate"
                value={`${selectedPool.fee_bp / 100}%`}
                icon={<Percent className="w-3 h-3" />}
                color="text-accent"
              />
              <DetailStat
                label="Trustlines"
                value={selectedPool.total_trustlines.toLocaleString()}
                icon={<Activity className="w-3 h-3" />}
                color="text-cyan-400"
              />
            </div>

            {/* Volume & Fees */}
            <div className="space-y-2">
              <div className="flex justify-between items-center p-3 rounded-xl bg-slate-900/20 border border-white/5">
                <span className="text-[10px] font-mono text-muted-foreground uppercase">
                  24h Volume
                </span>
                <span className="text-sm font-mono font-bold text-indigo-400">
                  {formatCurrency(selectedPool.volume_24h_usd)}
                </span>
              </div>
              <div className="flex justify-between items-center p-3 rounded-xl bg-slate-900/20 border border-white/5">
                <span className="text-[10px] font-mono text-muted-foreground uppercase">
                  24h Fees
                </span>
                <span className="text-sm font-mono font-bold text-amber-400">
                  {formatCurrency(selectedPool.fees_earned_24h_usd)}
                </span>
              </div>
              <div className="flex justify-between items-center p-3 rounded-xl bg-slate-900/20 border border-white/5">
                <span className="text-[10px] font-mono text-muted-foreground uppercase">
                  Total Value
                </span>
                <span className="text-sm font-mono font-bold text-foreground">
                  {formatCurrency(selectedPool.total_value_usd)}
                </span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Pool Comparison Summary */}
      <div className="glass-card rounded-2xl p-6 border border-border/50">
        <div className="flex items-center justify-between mb-6">
          <div>
            <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-1">
              Comparison Matrix // Quick View
            </div>
            <h3 className="text-lg font-black tracking-tight uppercase italic">
              Pool Efficiency Index
            </h3>
          </div>
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-border/50"
          >
            {pools.length} POOLS
          </Badge>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-3">
          {sortedPools.map((pool, idx) => (
            <div
              key={pool.pool_id}
              onClick={() => handleSelectPool(pool)}
              className={`p-4 rounded-xl border cursor-pointer transition-all duration-200 hover:scale-[1.02] ${
                selectedPool?.pool_id === pool.pool_id
                  ? "border-accent/50 bg-accent/10"
                  : "border-border/20 bg-slate-900/20 hover:border-accent/20"
              }`}
            >
              <div className="flex items-center gap-2 mb-3">
                <div className="text-[10px] font-mono text-muted-foreground/50">
                  #{idx + 1}
                </div>
                <div className="text-xs font-black uppercase">
                  {pool.reserve_a_asset_code}/{pool.reserve_b_asset_code}
                </div>
              </div>
              <div className="text-lg font-black font-mono text-emerald-400 mb-1">
                {formatPercent(pool.apy)}
              </div>
              <div className="text-[9px] font-mono text-muted-foreground uppercase">
                APY
              </div>
              <div className="mt-2 flex justify-between">
                <span className="text-[9px] font-mono text-muted-foreground">
                  IL:{" "}
                  <span className={getILSeverity(pool.impermanent_loss_pct)}>
                    {formatPercent(pool.impermanent_loss_pct)}
                  </span>
                </span>
                <span className="text-[9px] font-mono text-muted-foreground">
                  {formatCurrency(pool.total_value_usd)}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// =============================================================================
// Sub-components
// =============================================================================

function SortableHeader({
  label,
  sortKey,
  currentKey,
  ascending,
  onClick,
}: {
  label: string;
  sortKey: SortKey;
  currentKey: SortKey;
  ascending: boolean;
  onClick: (key: SortKey) => void;
}) {
  const isActive = currentKey === sortKey;
  return (
    <th
      onClick={() => onClick(sortKey)}
      className="text-right px-6 py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold cursor-pointer hover:text-accent transition-colors select-none"
    >
      <span className="inline-flex items-center gap-1">
        {label}
        {isActive && (
          <ChevronDown
            className={`w-3 h-3 transition-transform ${ascending ? "rotate-180" : ""}`}
          />
        )}
      </span>
    </th>
  );
}

function DetailStat({
  label,
  value,
  icon,
  color,
}: {
  label: string;
  value: string;
  icon: React.ReactNode;
  color: string;
}) {
  return (
    <div className="p-3 rounded-xl bg-slate-900/30 border border-white/5">
      <div className="flex items-center gap-1 mb-1">
        <span className={`${color}`}>{icon}</span>
        <span className="text-[9px] font-mono text-muted-foreground uppercase tracking-wider">
          {label}
        </span>
      </div>
      <div className={`text-sm font-mono font-bold ${color}`}>{value}</div>
    </div>
  );
}
