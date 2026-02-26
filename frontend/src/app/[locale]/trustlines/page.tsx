"use client";

import React, { useState, useEffect } from "react";
import {
  Users,
  TrendingUp,
  Activity,
  BarChart3,
  ArrowUpRight,
  ChevronRight,
  BadgeCheck,
} from "lucide-react";
import { TrustlineGrowthChart } from "@/components/charts/TrustlineGrowthChart";
import {
  TrustlineStat,
  TrustlineSnapshot,
  TrustlineMetrics,
  fetchTrustlineStats,
  fetchTrustlineRankings,
  fetchTrustlineHistory,
} from "@/lib/trustline-api";

export default function TrustlinesPage() {
  const [stats, setStats] = useState<TrustlineMetrics | null>(null);
  const [rankings, setRankings] = useState<TrustlineStat[]>([]);
  const [selectedAsset, setSelectedAsset] = useState<TrustlineStat | null>(
    null,
  );
  const [history, setHistory] = useState<TrustlineSnapshot[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function loadData() {
      setLoading(true);
      try {
        const [statsData, rankingsData] = await Promise.all([
          fetchTrustlineStats(),
          fetchTrustlineRankings(50),
        ]);

        setStats(statsData);
        setRankings(rankingsData);

        if (rankingsData.length > 0) {
          const topAsset = rankingsData[0];
          setSelectedAsset(topAsset);
          const historyData = await fetchTrustlineHistory(
            topAsset.asset_code,
            topAsset.asset_issuer,
          );
          setHistory(historyData);
        }
      } catch (error) {
        console.error("Error loading trustline data:", error);
      } finally {
        setLoading(false);
      }
    }

    loadData();
  }, []);

  const handleSelectAsset = async (asset: TrustlineStat) => {
    setSelectedAsset(asset);
    const historyData = await fetchTrustlineHistory(
      asset.asset_code,
      asset.asset_issuer,
    );
    setHistory(historyData);
  };

  const formatNumber = (value: number) => {
    return new Intl.NumberFormat("en-US", {
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(value);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="flex flex-col items-center gap-4">
          <div className="w-12 h-12 border-4 border-accent/20 border-t-accent rounded-full animate-spin glow-accent" />
          <p className="text-sm font-mono text-muted-foreground uppercase tracking-widest">
            Syncing Ledger States...
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 pt-4 sm:pt-6 space-y-6 sm:space-y-8 animate-in fade-in duration-700 pb-12">
      {/* Header */}
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-accent/20 rounded-xl flex items-center justify-center border border-accent/50 glow-accent shrink-0">
            <Users className="w-5 h-5 text-accent" />
          </div>
          <h1 className="text-3xl md:text-4xl font-black tracking-tighter uppercase italic">
            Trustline
            <span className="text-accent underline decoration-accent/30 decoration-4 underline-offset-4 ml-2">
              Analysis
            </span>
          </h1>
        </div>
        <p className="text-xs sm:text-sm font-mono text-muted-foreground uppercase tracking-widest mt-2 md:mt-0 pl-1 md:pl-14">
          Monitor asset adoption, holder distribution, and network growth
        </p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="glass-card rounded-2xl p-6 border border-border/50 flex flex-col justify-between group hover:border-accent/50 transition-colors relative overflow-hidden">
          <div className="absolute top-0 right-0 w-32 h-32 bg-accent/5 rounded-full blur-3xl -mr-10 -mt-10 transition-all group-hover:bg-accent/10" />
          <div className="flex items-center justify-between mb-4 relative z-10">
            <div className="flex items-center gap-2">
              <Users className="w-4 h-4 text-emerald-400" />
              <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider">
                Total Trustlines
              </span>
            </div>
          </div>
          <div className="relative z-10">
            <div className="text-3xl font-black font-mono tracking-tighter mb-1">
              {formatNumber(stats?.total_trustlines_across_network || 0)}
            </div>
            <div className="text-xs font-mono text-emerald-400/80 flex items-center gap-1">
              <ArrowUpRight className="w-3 h-3" /> System-wide tracked
            </div>
          </div>
        </div>

        <div className="glass-card rounded-2xl p-6 border border-border/50 flex flex-col justify-between group hover:border-accent/50 transition-colors relative overflow-hidden">
          <div className="absolute top-0 right-0 w-32 h-32 bg-accent/5 rounded-full blur-3xl -mr-10 -mt-10 transition-all group-hover:bg-accent/10" />
          <div className="flex items-center justify-between mb-4 relative z-10">
            <div className="flex items-center gap-2">
              <Activity className="w-4 h-4 text-emerald-400" />
              <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider">
                Tracked Assets
              </span>
            </div>
          </div>
          <div className="relative z-10">
            <div className="text-3xl font-black font-mono tracking-tighter mb-1">
              {formatNumber(stats?.total_assets_tracked || 0)}
            </div>
            <div className="text-xs font-mono text-muted-foreground flex items-center gap-1">
              Monitoring verified issuers
            </div>
          </div>
        </div>

        <div className="glass-card rounded-2xl p-6 border border-border/50 flex flex-col justify-between group hover:border-accent/50 transition-colors relative overflow-hidden">
          <div className="absolute top-0 right-0 w-32 h-32 bg-accent/5 rounded-full blur-3xl -mr-10 -mt-10 transition-all group-hover:bg-accent/10" />
          <div className="flex items-center justify-between mb-4 relative z-10">
            <div className="flex items-center gap-2">
              <TrendingUp className="w-4 h-4 text-emerald-400" />
              <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider">
                Growth Trend
              </span>
            </div>
          </div>
          <div className="relative z-10">
            <div className="text-3xl font-black font-mono tracking-tighter mb-1 text-emerald-400">
              Positive
            </div>
            <div className="text-xs font-mono text-muted-foreground flex items-center gap-1">
              Based on rolling 30-day average
            </div>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Rankings Table */}
        <div className="lg:col-span-1 glass-card rounded-2xl border border-border/50 flex flex-col overflow-hidden h-[600px]">
          <div className="p-4 border-b border-white/5 bg-slate-900/50 flex items-center justify-between shrink-0">
            <div className="flex items-center gap-2">
              <BarChart3 className="w-4 h-4 text-accent" />
              <h2 className="text-sm font-bold uppercase tracking-wider">
                Top Assets
              </h2>
            </div>
          </div>

          <div className="overflow-y-auto flex-1 p-2 space-y-1 custom-scrollbar">
            {rankings.map((asset, index) => {
              const isSelected = selectedAsset?.asset_code === asset.asset_code;
              return (
                <button
                  key={`${asset.asset_code}-${asset.asset_issuer}`}
                  onClick={() => handleSelectAsset(asset)}
                  className={`w-full flex items-center justify-between p-3 rounded-xl transition-all ${
                    isSelected
                      ? "bg-accent/10 border border-accent/20"
                      : "hover:bg-white/5 border border-transparent"
                  }`}
                >
                  <div className="flex items-center gap-3 text-left">
                    <div className="w-6 text-xs font-mono text-muted-foreground">
                      {(index + 1).toString().padStart(2, "0")}
                    </div>
                    <div>
                      <div className="font-bold flex items-center gap-1">
                        {asset.asset_code}
                        {index < 10 && (
                          <BadgeCheck className="w-3 h-3 text-emerald-400" />
                        )}
                      </div>
                      <div className="text-[10px] font-mono text-muted-foreground truncate w-24">
                        {asset.asset_issuer.slice(0, 4)}...
                        {asset.asset_issuer.slice(-4)}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-3">
                    <div className="text-right">
                      <div className="font-mono text-sm">
                        {formatNumber(asset.total_trustlines)}
                      </div>
                      <div className="text-[9px] font-mono text-emerald-400 uppercase">
                        Trustlines
                      </div>
                    </div>
                    <ChevronRight
                      className={`w-4 h-4 ${isSelected ? "text-accent" : "text-muted-foreground/30"}`}
                    />
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Selected Asset Details */}
        <div className="lg:col-span-2 space-y-6">
          {selectedAsset ? (
            <>
              {/* Asset Header Info */}
              <div className="glass-card rounded-2xl p-6 border border-border/50 flex flex-col sm:flex-row justify-between items-start gap-4 sm:gap-0">
                <div>
                  <h2 className="text-2xl sm:text-3xl font-black tracking-tighter flex items-center gap-2">
                    {selectedAsset.asset_code}
                    <BadgeCheck className="w-5 h-5 sm:w-6 sm:h-6 text-emerald-400 inline-block mb-1" />
                  </h2>
                  <p className="text-[10px] sm:text-xs font-mono text-muted-foreground mt-1 break-all">
                    Issuer: {selectedAsset.asset_issuer}
                  </p>
                </div>
                <div className="text-left sm:text-right">
                  <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mb-1">
                    Total Supply
                  </div>
                  <div className="text-lg sm:text-xl font-black font-mono tracking-tighter">
                    {formatNumber(selectedAsset.total_supply)}
                  </div>
                </div>
              </div>

              {/* Chart */}
              <TrustlineGrowthChart
                data={history}
                latestTotal={selectedAsset.total_trustlines}
              />

              {/* Distribution */}
              <div className="glass-card rounded-2xl p-6 border border-border/50">
                <h3 className="text-sm font-bold uppercase tracking-wider mb-6">
                  Holder Distribution
                </h3>

                <div className="space-y-4">
                  <div className="flex flex-col sm:flex-row justify-between text-sm font-mono mb-2 gap-2 sm:gap-0">
                    <span className="text-emerald-400 text-xs flex items-center gap-2">
                      <div className="w-2 h-2 rounded bg-emerald-400 shrink-0" />{" "}
                      Authorized (
                      {formatNumber(selectedAsset.authorized_trustlines)})
                    </span>
                    <span className="text-rose-400 text-xs flex items-center justify-start sm:justify-end gap-2">
                      Unauthorized (
                      {formatNumber(selectedAsset.unauthorized_trustlines)}){" "}
                      <div className="w-2 h-2 rounded bg-rose-400 shrink-0" />
                    </span>
                  </div>

                  {/* Progress Bar */}
                  <div className="h-3 w-full bg-slate-800 rounded-full overflow-hidden flex">
                    <div
                      className="h-full bg-emerald-400"
                      style={{
                        width: `${(selectedAsset.authorized_trustlines / selectedAsset.total_trustlines) * 100}%`,
                      }}
                    />
                    <div
                      className="h-full bg-rose-400"
                      style={{
                        width: `${(selectedAsset.unauthorized_trustlines / selectedAsset.total_trustlines) * 100}%`,
                      }}
                    />
                  </div>

                  <p className="text-[10px] font-mono text-muted-foreground uppercase text-center mt-2">
                    {selectedAsset.unauthorized_trustlines > 0
                      ? "Includes trustlines pending authorization or revoked"
                      : "All trustlines are fully authorized"}
                  </p>
                </div>
              </div>
            </>
          ) : (
            <div className="glass-card rounded-2xl p-6 border border-border/50 flex flex-col items-center justify-center h-full">
              <Users className="w-12 h-12 text-muted-foreground/30 mb-4" />
              <p className="text-sm font-mono text-muted-foreground uppercase tracking-widest text-center">
                Select an asset from the leaderboard
                <br />
                to view detailed insights
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
