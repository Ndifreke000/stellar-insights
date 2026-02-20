"use client";

import React, { useState, useEffect } from "react";
import {
  Link2,
  TrendingUp,
  BarChart3,
  ArrowUpRight,
  ArrowDownRight,
  Shield,
  Users,
  ChevronDown,
  Layers,
  Database,
} from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { TrustlineGrowthChart } from "@/components/charts/TrustlineGrowthChart";
import { Badge } from "@/components/ui/badge";
import {
  TrustlineAsset,
  TrustlineSnapshot,
  TrustlineStats,
  TrustlineChange,
  fetchTrustlineAssets,
  fetchTrustlineStats,
  fetchTrustlineChanges,
  fetchAssetSnapshots,
} from "@/lib/trustline-api";

type SortKey = "num_accounts" | "amount";

export default function TrustlinesPage() {
  const [loading, setLoading] = useState(true);
  const [assets, setAssets] = useState<TrustlineAsset[]>([]);
  const [stats, setStats] = useState<TrustlineStats | null>(null);
  const [changes, setChanges] = useState<TrustlineChange[]>([]);
  const [selectedAsset, setSelectedAsset] = useState<TrustlineAsset | null>(
    null,
  );
  const [snapshots, setSnapshots] = useState<TrustlineSnapshot[]>([]);
  const [sortKey, setSortKey] = useState<SortKey>("num_accounts");
  const [sortAsc, setSortAsc] = useState(false);

  useEffect(() => {
    async function load() {
      const [assetsData, statsData, changesData] = await Promise.all([
        fetchTrustlineAssets(),
        fetchTrustlineStats(),
        fetchTrustlineChanges(),
      ]);
      setAssets(assetsData);
      setStats(statsData);
      setChanges(changesData);

      if (assetsData.length > 0) {
        setSelectedAsset(assetsData[0]);
        const assetKey = `${assetsData[0].asset_code}:${assetsData[0].asset_issuer}`;
        const snaps = await fetchAssetSnapshots(assetKey);
        setSnapshots(snaps);
      }
      setLoading(false);
    }
    load();
  }, []);

  const handleSelectAsset = async (asset: TrustlineAsset) => {
    setSelectedAsset(asset);
    const assetKey = `${asset.asset_code}:${asset.asset_issuer}`;
    const snaps = await fetchAssetSnapshots(assetKey);
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

  const sortedAssets = [...assets].sort((a, b) => {
    const aVal = a[sortKey];
    const bVal = b[sortKey];
    return sortAsc
      ? (aVal as number) - (bVal as number)
      : (bVal as number) - (aVal as number);
  });

  const formatNumber = (value: number) =>
    new Intl.NumberFormat("en-US", {
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(value);

  const formatSupply = (value: number) =>
    new Intl.NumberFormat("en-US", {
      notation: "compact",
      maximumFractionDigits: 2,
    }).format(value);

  const truncateIssuer = (issuer: string) =>
    `${issuer.slice(0, 4)}...${issuer.slice(-4)}`;

  if (loading) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest italic">
          Scanning Trustlines... // 404-TL
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-5 md:space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      {/* Page Header */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-3 md:gap-4 border-b border-border/50 pb-4 md:pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-1 md:mb-2">
            Asset Analytics // 07
          </div>
          <h2 className="text-2xl md:text-4xl font-black tracking-tighter uppercase italic flex items-center gap-2 md:gap-3">
            <Link2 className="w-6 h-6 md:w-8 md:h-8 text-accent flex-shrink-0" />
            Trustline Dashboard
          </h2>
          <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mt-1 md:mt-2">
            Asset adoption, trustline growth, and holder distribution analytics
          </p>
        </div>
        <div className="flex items-center gap-2 md:gap-3">
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-border/50 px-2 md:px-3 py-1 bg-accent/5"
          >
            {assets.length} TRACKED_ASSETS
          </Badge>
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-emerald-500/30 px-2 md:px-3 py-1 bg-emerald-500/5 text-emerald-400"
          >
            LIVE_FEED
          </Badge>
        </div>
      </div>

      {/* Overview Metrics */}
      {stats && (
        <div className="grid grid-cols-2 sm:grid-cols-2 lg:grid-cols-4 gap-2 md:gap-4">
          <MetricCard
            label="Total Assets Tracked"
            value={stats.total_assets.toLocaleString()}
            subLabel="Stellar Network Assets"
            trend={4.2}
            trendDirection="up"
          />
          <MetricCard
            label="Total Trustlines"
            value={formatNumber(stats.total_trustlines)}
            subLabel="Across All Assets"
            trend={2.8}
            trendDirection="up"
          />
          <MetricCard
            label="Avg Trustlines"
            value={formatNumber(stats.avg_trustlines)}
            subLabel="Per Asset"
            trend={1.5}
            trendDirection="up"
          />
          <MetricCard
            label="Growing Assets"
            value={`${stats.assets_growing} / ${stats.total_assets}`}
            subLabel={`${stats.assets_declining} declining`}
            trend={stats.assets_growing > stats.assets_declining ? 3.1 : -1.2}
            trendDirection={
              stats.assets_growing > stats.assets_declining ? "up" : "down"
            }
          />
        </div>
      )}

      {/* Asset Rankings Table */}
      <div className="glass-card rounded-xl md:rounded-2xl p-1 border border-border/50">
        <div className="p-4 md:p-6 pb-3 md:pb-4">
          <h3 className="text-xs font-mono text-muted-foreground uppercase tracking-widest mb-1 flex items-center gap-2">
            <BarChart3 className="w-3 h-3 text-accent" />
            Asset Rankings
          </h3>
          <p className="text-[10px] font-mono text-muted-foreground/50 uppercase tracking-wider">
            Tap an asset to view its growth chart
          </p>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-xs font-mono">
            <thead>
              <tr className="border-b border-border/30">
                <th className="text-left px-3 md:px-6 py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                  #
                </th>
                <th className="text-left px-3 md:px-6 py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                  Asset
                </th>
                <SortableHeader
                  label="Trustlines"
                  sortKey="num_accounts"
                  currentKey={sortKey}
                  ascending={sortAsc}
                  onClick={handleSort}
                />
                <SortableHeader
                  label="Supply"
                  sortKey="amount"
                  currentKey={sortKey}
                  ascending={sortAsc}
                  onClick={handleSort}
                />
                <th className="hidden sm:table-cell text-right px-3 md:px-6 py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                  Auth
                </th>
                <th className="hidden md:table-cell text-right px-3 md:px-6 py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                  Issuer
                </th>
              </tr>
            </thead>
            <tbody>
              {sortedAssets.map((asset, idx) => (
                <tr
                  key={`${asset.asset_code}:${asset.asset_issuer}`}
                  onClick={() => handleSelectAsset(asset)}
                  className={`border-b border-border/10 cursor-pointer transition-all duration-200 hover:bg-accent/5 ${
                    selectedAsset?.asset_code === asset.asset_code &&
                    selectedAsset?.asset_issuer === asset.asset_issuer
                      ? "bg-accent/10 border-accent/20"
                      : ""
                  }`}
                >
                  <td className="px-3 md:px-6 py-3 md:py-4 text-muted-foreground/50">
                    {idx + 1}
                  </td>
                  <td className="px-3 md:px-6 py-3 md:py-4">
                    <div className="flex items-center gap-2">
                      <div className="w-6 h-6 md:w-7 md:h-7 rounded-full bg-gradient-to-br from-accent/30 to-accent/10 flex items-center justify-center text-[9px] font-black text-accent flex-shrink-0">
                        {asset.asset_code.charAt(0)}
                      </div>
                      <div className="min-w-0">
                        <div className="font-bold text-foreground text-xs">
                          {asset.asset_code}
                        </div>
                        <div className="text-[9px] text-muted-foreground/50 hidden sm:block">
                          {asset.asset_type}
                        </div>
                      </div>
                    </div>
                  </td>
                  <td className="px-3 md:px-6 py-3 md:py-4 text-right">
                    <span className="font-bold text-indigo-400">
                      {asset.num_accounts.toLocaleString()}
                    </span>
                  </td>
                  <td className="px-3 md:px-6 py-3 md:py-4 text-right font-bold">
                    {formatSupply(asset.amount)}
                  </td>
                  <td className="hidden sm:table-cell px-3 md:px-6 py-3 md:py-4 text-right">
                    <div className="flex items-center justify-end gap-1">
                      {asset.flags_auth_required ? (
                        <Badge
                          variant="outline"
                          className="text-[8px] font-mono border-amber-500/30 text-amber-400 px-1.5 py-0"
                        >
                          REQ
                        </Badge>
                      ) : null}
                      {asset.flags_auth_revocable ? (
                        <Badge
                          variant="outline"
                          className="text-[8px] font-mono border-red-500/30 text-red-400 px-1.5 py-0"
                        >
                          REV
                        </Badge>
                      ) : null}
                      {asset.flags_auth_clawback ? (
                        <Badge
                          variant="outline"
                          className="text-[8px] font-mono border-purple-500/30 text-purple-400 px-1.5 py-0"
                        >
                          CLW
                        </Badge>
                      ) : null}
                      {!asset.flags_auth_required &&
                        !asset.flags_auth_revocable &&
                        !asset.flags_auth_clawback && (
                          <span className="text-[9px] text-muted-foreground/40">
                            OPEN
                          </span>
                        )}
                    </div>
                  </td>
                  <td className="hidden md:table-cell px-3 md:px-6 py-3 md:py-4 text-right text-[10px] text-muted-foreground/60">
                    {truncateIssuer(asset.asset_issuer)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Growth Chart + Asset Detail */}
      {selectedAsset && (
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-4 md:gap-6">
          <div className="lg:col-span-8">
            <TrustlineGrowthChart
              snapshots={snapshots}
              assetCode={selectedAsset.asset_code}
            />
          </div>

          {/* Asset Detail Panel */}
          <div className="lg:col-span-4 glass-card rounded-xl md:rounded-2xl p-4 md:p-6 border border-border/50 space-y-4 md:space-y-6">
            <div>
              <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
                Selected Asset // Detail
              </div>
              <h3 className="text-2xl font-black tracking-tighter uppercase italic">
                {selectedAsset.asset_code}
              </h3>
              <p className="text-[9px] font-mono text-muted-foreground/50 mt-1 break-all">
                {selectedAsset.asset_issuer}
              </p>
            </div>

            {/* Key Metrics */}
            <div className="grid grid-cols-2 gap-3">
              <DetailStat
                label="Trustlines"
                value={selectedAsset.num_accounts.toLocaleString()}
                icon={<Users className="w-3 h-3" />}
                color="text-indigo-400"
              />
              <DetailStat
                label="Supply"
                value={formatSupply(selectedAsset.amount)}
                icon={<Layers className="w-3 h-3" />}
                color="text-emerald-400"
              />
              <DetailStat
                label="Authorized"
                value={selectedAsset.num_accounts_authorized.toLocaleString()}
                icon={<Shield className="w-3 h-3" />}
                color="text-cyan-400"
              />
              <DetailStat
                label="Type"
                value={selectedAsset.asset_type.replace("credit_alphanum", "α")}
                icon={<Database className="w-3 h-3" />}
                color="text-amber-400"
              />
            </div>

            {/* Auth Flags */}
            <div className="space-y-2">
              <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest">
                Authorization Flags
              </div>
              <div className="grid grid-cols-2 gap-2">
                <FlagIndicator
                  label="Auth Required"
                  active={!!selectedAsset.flags_auth_required}
                />
                <FlagIndicator
                  label="Auth Revocable"
                  active={!!selectedAsset.flags_auth_revocable}
                />
                <FlagIndicator
                  label="Auth Immutable"
                  active={!!selectedAsset.flags_auth_immutable}
                />
                <FlagIndicator
                  label="Clawback"
                  active={!!selectedAsset.flags_auth_clawback}
                />
              </div>
            </div>

            {/* Summary Stats */}
            <div className="space-y-2">
              <div className="flex justify-between items-center p-3 rounded-xl bg-slate-900/20 border border-white/5">
                <span className="text-[10px] font-mono text-muted-foreground uppercase">
                  Holders
                </span>
                <span className="text-sm font-mono font-bold text-indigo-400">
                  {selectedAsset.num_accounts.toLocaleString()}
                </span>
              </div>
              <div className="flex justify-between items-center p-3 rounded-xl bg-slate-900/20 border border-white/5">
                <span className="text-[10px] font-mono text-muted-foreground uppercase">
                  Circulating Supply
                </span>
                <span className="text-sm font-mono font-bold text-emerald-400">
                  {formatSupply(selectedAsset.amount)}
                </span>
              </div>
              <div className="flex justify-between items-center p-3 rounded-xl bg-slate-900/20 border border-white/5">
                <span className="text-[10px] font-mono text-muted-foreground uppercase">
                  Avg per Holder
                </span>
                <span className="text-sm font-mono font-bold text-foreground">
                  {formatSupply(
                    selectedAsset.num_accounts > 0
                      ? selectedAsset.amount / selectedAsset.num_accounts
                      : 0,
                  )}
                </span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Recent Changes */}
      <div className="glass-card rounded-xl md:rounded-2xl p-4 md:p-6 border border-border/50">
        <div className="flex items-center justify-between mb-4 md:mb-6">
          <div>
            <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-1">
              Trustline Changes // Delta
            </div>
            <h3 className="text-base md:text-lg font-black tracking-tight uppercase italic">
              Recent Activity
            </h3>
          </div>
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-border/50"
          >
            {changes.length} CHANGES
          </Badge>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2 md:gap-3">
          {changes.map((change) => (
            <div
              key={`${change.asset_code}:${change.asset_issuer}`}
              className="p-4 rounded-xl border border-border/20 bg-slate-900/20 hover:border-accent/20 transition-all duration-200"
            >
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-2">
                  <div className="w-6 h-6 rounded-full bg-gradient-to-br from-accent/30 to-accent/10 flex items-center justify-center text-[8px] font-black text-accent">
                    {change.asset_code.charAt(0)}
                  </div>
                  <span className="text-sm font-black uppercase">
                    {change.asset_code}
                  </span>
                </div>
                <div
                  className={`flex items-center gap-1 text-xs font-mono font-bold ${
                    change.change_amount >= 0
                      ? "text-emerald-400"
                      : "text-red-400"
                  }`}
                >
                  {change.change_amount >= 0 ? (
                    <ArrowUpRight className="w-3 h-3" />
                  ) : (
                    <ArrowDownRight className="w-3 h-3" />
                  )}
                  {change.change_pct >= 0 ? "+" : ""}
                  {change.change_pct.toFixed(2)}%
                </div>
              </div>
              <div className="flex justify-between items-end">
                <div>
                  <div className="text-[9px] font-mono text-muted-foreground uppercase">
                    Change
                  </div>
                  <div
                    className={`text-lg font-mono font-bold ${
                      change.change_amount >= 0
                        ? "text-emerald-400"
                        : "text-red-400"
                    }`}
                  >
                    {change.change_amount >= 0 ? "+" : ""}
                    {change.change_amount.toLocaleString()}
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-[9px] font-mono text-muted-foreground uppercase">
                    Current
                  </div>
                  <div className="text-sm font-mono font-bold text-foreground">
                    {change.current_accounts.toLocaleString()}
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Asset Adoption Grid */}
      <div className="glass-card rounded-xl md:rounded-2xl p-4 md:p-6 border border-border/50">
        <div className="flex items-center justify-between mb-4 md:mb-6">
          <div>
            <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-1">
              Adoption Overview // Grid
            </div>
            <h3 className="text-base md:text-lg font-black tracking-tight uppercase italic">
              Asset Adoption Index
            </h3>
          </div>
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-border/50"
          >
            {assets.length} ASSETS
          </Badge>
        </div>
        <div className="grid grid-cols-2 sm:grid-cols-2 lg:grid-cols-4 gap-2 md:gap-3">
          {sortedAssets.map((asset, idx) => (
            <div
              key={`grid-${asset.asset_code}:${asset.asset_issuer}`}
              onClick={() => handleSelectAsset(asset)}
              className={`p-4 rounded-xl border cursor-pointer transition-all duration-200 hover:scale-[1.02] ${
                selectedAsset?.asset_code === asset.asset_code &&
                selectedAsset?.asset_issuer === asset.asset_issuer
                  ? "border-accent/50 bg-accent/10"
                  : "border-border/20 bg-slate-900/20 hover:border-accent/20"
              }`}
            >
              <div className="flex items-center gap-2 mb-3">
                <div className="text-[10px] font-mono text-muted-foreground/50">
                  #{idx + 1}
                </div>
                <div className="text-xs font-black uppercase">
                  {asset.asset_code}
                </div>
              </div>
              <div className="text-lg font-black font-mono text-indigo-400 mb-1">
                {formatNumber(asset.num_accounts)}
              </div>
              <div className="text-[9px] font-mono text-muted-foreground uppercase">
                Trustlines
              </div>
              <div className="mt-2 flex justify-between">
                <span className="text-[9px] font-mono text-muted-foreground">
                  Supply:{" "}
                  <span className="text-emerald-400">
                    {formatSupply(asset.amount)}
                  </span>
                </span>
                <span className="text-[9px] font-mono text-muted-foreground">
                  {asset.flags_auth_required ? "🔒" : "🔓"}
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
      className="text-right px-3 md:px-6 py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold cursor-pointer hover:text-accent transition-colors select-none"
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

function FlagIndicator({ label, active }: { label: string; active: boolean }) {
  return (
    <div className="flex items-center gap-2 p-2 rounded-lg bg-slate-900/20 border border-white/5">
      <div
        className={`w-2 h-2 rounded-full ${active ? "bg-amber-400 shadow-[0_0_6px_rgba(251,191,36,0.5)]" : "bg-slate-600"}`}
      />
      <span className="text-[9px] font-mono text-muted-foreground uppercase">
        {label}
      </span>
    </div>
  );
}
