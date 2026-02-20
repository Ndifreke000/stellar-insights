"use client";

import React, { useState, useEffect } from "react";
import {
  Wallet,
  Clock,
  AlertTriangle,
  BarChart3,
  ChevronDown,
  Filter,
} from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { ClaimableBalanceCard } from "@/components/ClaimableBalanceCard";
import { Badge } from "@/components/ui/badge";
import {
  ClaimableBalance,
  ClaimableBalanceAnalytics,
  fetchClaimableBalances,
  fetchClaimableBalanceAnalytics,
  fetchExpiringBalances,
} from "@/lib/claimable-balance-api";

export default function ClaimableBalancesPage() {
  const [loading, setLoading] = useState(true);
  const [balances, setBalances] = useState<ClaimableBalance[]>([]);
  const [analytics, setAnalytics] = useState<ClaimableBalanceAnalytics | null>(null);
  const [expiring, setExpiring] = useState<ClaimableBalance[]>([]);
  const [filterActive, setFilterActive] = useState<boolean | null>(null);
  const [filterAsset, setFilterAsset] = useState<string>("");
  const [showFilters, setShowFilters] = useState(false);

  useEffect(() => {
    async function load() {
      setLoading(true);
      const [balancesData, analyticsData, expiringData] = await Promise.all([
        fetchClaimableBalances({
          claimed: filterActive ?? undefined,
          asset_code: filterAsset || undefined,
          limit: 100,
        }),
        fetchClaimableBalanceAnalytics(),
        fetchExpiringBalances(30),
      ]);
      setBalances(balancesData);
      setAnalytics(analyticsData);
      setExpiring(expiringData);
      setLoading(false);
    }
    load();
  }, [filterActive, filterAsset]);

  const formatCurrency = (value: number) =>
    new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(value);

  const uniqueAssets = Array.from(
    new Set(balances.map((b) => b.asset_code).filter(Boolean)),
  ).sort();

  if (loading) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest italic">
          Loading Claimable Balances... // CB-01
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
            Stellar Network // Escrow & Airdrops
          </div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <Wallet className="w-8 h-8 text-accent" />
            Claimable Balances
          </h2>
          <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mt-2">
            Track pending claims, expiration dates, and claim success rates
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-border/50 px-3 py-1 bg-accent/5"
          >
            {balances.length} PENDING
          </Badge>
          {analytics && analytics.expiring_soon_count > 0 && (
            <Badge
              variant="outline"
              className="text-[10px] font-mono border-amber-500/30 px-3 py-1 bg-amber-500/10 text-amber-400"
            >
              {analytics.expiring_soon_count} EXPIRING SOON
            </Badge>
          )}
        </div>
      </div>

      {/* Overview Metrics */}
      {analytics && (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <MetricCard
            label="Total Locked"
            value={analytics.total_locked_count.toLocaleString()}
            subLabel="Claimable balances tracked"
          />
          <MetricCard
            label="Pending Claims"
            value={analytics.pending_claims_count.toLocaleString()}
            subLabel="Unclaimed balances"
          />
          <MetricCard
            label="Expiring Soon"
            value={analytics.expiring_soon_count.toLocaleString()}
            subLabel="Within 30 days"
          />
          <MetricCard
            label="Claim Success Rate"
            value={`${analytics.claim_success_rate.toFixed(1)}%`}
            subLabel="Historical claim rate"
          />
        </div>
      )}

      {/* Filters */}
      <div className="glass-card rounded-2xl p-4 border border-border/50">
        <button
          onClick={() => setShowFilters(!showFilters)}
          className="flex items-center gap-2 text-[10px] font-mono text-muted-foreground uppercase tracking-widest hover:text-accent transition-colors"
        >
          <Filter className="w-3 h-3" />
          Filters
          <ChevronDown
            className={`w-4 h-4 transition-transform ${showFilters ? "rotate-180" : ""}`}
          />
        </button>
        {showFilters && (
          <div className="mt-4 flex flex-wrap gap-4">
            <div>
              <label className="text-[9px] font-mono text-muted-foreground uppercase block mb-1">
                Status
              </label>
              <select
                value={filterActive === null ? "all" : filterActive ? "claimed" : "active"}
                onChange={(e) => {
                  const v = e.target.value;
                  setFilterActive(v === "all" ? null : v === "claimed");
                }}
                className="rounded-lg border border-border/50 bg-slate-900/50 px-3 py-2 text-xs font-mono"
              >
                <option value="all">All</option>
                <option value="active">Active (unclaimed)</option>
                <option value="claimed">Claimed</option>
              </select>
            </div>
            <div>
              <label className="text-[9px] font-mono text-muted-foreground uppercase block mb-1">
                Asset
              </label>
              <select
                value={filterAsset}
                onChange={(e) => setFilterAsset(e.target.value)}
                className="rounded-lg border border-border/50 bg-slate-900/50 px-3 py-2 text-xs font-mono min-w-[120px]"
              >
                <option value="">All Assets</option>
                {uniqueAssets.map((code) => (
                  <option key={code} value={code}>
                    {code}
                  </option>
                ))}
              </select>
            </div>
          </div>
        )}
      </div>

      {/* Expiring Soon Section */}
      {expiring.length > 0 && (
        <div className="glass-card rounded-2xl p-6 border border-amber-500/30">
          <div className="flex items-center gap-2 mb-4">
            <AlertTriangle className="w-5 h-5 text-amber-400" />
            <h3 className="text-sm font-mono font-bold uppercase tracking-widest text-amber-400">
              Expiring Within 30 Days
            </h3>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {expiring.slice(0, 6).map((b) => (
              <ClaimableBalanceCard key={b.id} balance={b} />
            ))}
          </div>
        </div>
      )}

      {/* All Balances */}
      <div className="glass-card rounded-2xl p-6 border border-border/50">
        <div className="flex items-center gap-2 mb-4">
          <BarChart3 className="w-5 h-5 text-accent" />
          <h3 className="text-sm font-mono font-bold uppercase tracking-widest text-foreground">
            All Claimable Balances
          </h3>
        </div>
        {balances.length === 0 ? (
          <div className="py-16 text-center">
            <Clock className="w-12 h-12 text-muted-foreground/30 mx-auto mb-4" />
            <p className="text-sm font-mono text-muted-foreground">
              No claimable balances found.
            </p>
            <p className="text-[10px] font-mono text-muted-foreground/70 mt-2">
              Balances sync from Horizon every 10 minutes.
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {balances.map((b) => (
              <ClaimableBalanceCard key={b.id} balance={b} />
            ))}
          </div>
        )}
      </div>

      {/* Top Assets */}
      {analytics && analytics.top_assets.length > 0 && (
        <div className="glass-card rounded-2xl p-6 border border-border/50">
          <h3 className="text-sm font-mono font-bold uppercase tracking-widest text-foreground mb-4">
            Top Assets by Locked Amount
          </h3>
          <div className="overflow-x-auto">
            <table className="w-full text-xs font-mono">
              <thead>
                <tr className="border-b border-border/30">
                  <th className="text-left py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                    Asset
                  </th>
                  <th className="text-right py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                    Total Amount
                  </th>
                  <th className="text-right py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                    Count
                  </th>
                </tr>
              </thead>
              <tbody>
                {analytics.top_assets.map((a) => (
                  <tr key={`${a.asset_code}-${a.asset_issuer || "native"}`} className="border-b border-border/10">
                    <td className="py-3 font-bold">
                      {a.asset_code}
                      {a.asset_issuer && (
                        <span className="text-muted-foreground/70 ml-1 text-[9px]">
                          ({a.asset_issuer.slice(0, 8)}...)
                        </span>
                      )}
                    </td>
                    <td className="py-3 text-right font-bold text-emerald-400">
                      {a.total_amount.toLocaleString()}
                    </td>
                    <td className="py-3 text-right text-muted-foreground">
                      {a.count}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
