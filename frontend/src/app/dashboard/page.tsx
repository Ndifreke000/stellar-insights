"use client";

import React, { useEffect, useState, useCallback } from "react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { CorridorHealth } from "@/components/dashboard/CorridorHealth";
import { LiquidityChart } from "@/components/dashboard/LiquidityChart";
import { TopAssetsTable } from "@/components/dashboard/TopAssetsTable";
import { SettlementSpeedChart } from "@/components/dashboard/SettlementSpeedChart";
import { WebSocketStatus } from "@/components/WebSocketStatus";
import { DataRefreshIndicator } from "@/components/DataRefreshIndicator";
import { useRealtimeCorridors } from "@/hooks/useRealtimeCorridors";
import { useRealtimeAnchors } from "@/hooks/useRealtimeAnchors";
import { useDataRefresh } from "@/hooks/useDataRefresh";

interface CorridorData {
  id: string;
  name: string;
  status: "optimal" | "degraded" | "down";
  uptime: number;
  volume24h: number;
}

interface LiquidityData {
  date: string;
  value: number;
}

interface AssetData {
  symbol: string;
  name: string;
  volume24h: number;
  price: number;
  change24h: number;
}

interface SettlementData {
  time: string;
  speed: number;
}

interface DashboardData {
  kpi: {
    successRate: {
      value: number;
      trend: number;
      trendDirection: "up" | "down";
    };
    activeCorridors: {
      value: number;
      trend: number;
      trendDirection: "up" | "down";
    };
    liquidityDepth: {
      value: number;
      trend: number;
      trendDirection: "up" | "down";
    };
    settlementSpeed: {
      value: number;
      trend: number;
      trendDirection: "up" | "down";
    };
  };
  corridors: CorridorData[];
  liquidity: LiquidityData[];
  assets: AssetData[];
  settlement: SettlementData[];
}

export default function DashboardPage() {
  const [data, setData] = useState<DashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // ── Data refresh hook (auto-refresh every 30 s + manual trigger) ──────────
  const fetchDashboard = useCallback(async () => {
    const response = await fetch("/api/dashboard");
    if (!response.ok) throw new Error("Failed to fetch dashboard data");
    const result = await response.json();
    setData(result);
  }, []);

  const {
    lastUpdated,
    secondsUntilRefresh,
    isRefreshing,
    triggerRefresh,
    markUpdated,
  } = useDataRefresh({
    refreshIntervalMs: 30_000,
    onRefresh: fetchDashboard,
  });

  // ── WebSocket connections for real-time updates ─────────────────────────
  const {
    isConnected: corridorsConnected,
    isConnecting: corridorsConnecting,
    connectionAttempts: corridorAttempts,
    reconnect: reconnectCorridors,
  } = useRealtimeCorridors({
    enablePaymentStream: true,
    onCorridorUpdate: (update) => {
      console.log("Received corridor update:", update);
      markUpdated();
      setData((prevData) => {
        if (!prevData) return prevData;
        const updatedData = { ...prevData };
        if (update.success_rate !== undefined) {
          updatedData.kpi.successRate.value = update.success_rate;
        }
        return updatedData;
      });
    },
    onHealthAlert: (alert) => {
      console.log("Health alert:", alert);
    },
  });

  const { isConnected: anchorsConnected, reconnect: reconnectAnchors } =
    useRealtimeAnchors({
      onAnchorUpdate: (update) => {
        console.log("Received anchor update:", update);
        markUpdated();
      },
    });

  // Initial load on mount
  useEffect(() => {
    (async () => {
      try {
        await fetchDashboard();
      } catch (err) {
        const isNetworkError =
          err instanceof TypeError &&
          (err.message.includes("Failed to fetch") ||
            err.message.includes("fetch is not defined") ||
            err.message.includes("Network request failed"));
        const errorMessage =
          err instanceof Error ? err.message : "An error occurred";
        setError(errorMessage);
        if (!isNetworkError) console.error("Dashboard API error:", err);
      } finally {
        setLoading(false);
      }
    })();
  }, [fetchDashboard]);

  if (loading) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest">
          Initialising Terminal... // System Handshake
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="px-6 py-4 glass border-red-500/50 text-red-500 font-mono text-sm uppercase tracking-widest">
          Terminal Error: {error}
        </div>
      </div>
    );
  }

  if (!data) return null;

  const formatVolume = (val: number) => {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(val);
  };

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            Intelligence Terminal // 01
          </div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic">
            Network Overview
          </h2>
        </div>
        <div className="flex items-center gap-2 flex-wrap">
          <WebSocketStatus
            isConnected={corridorsConnected && anchorsConnected}
            isConnecting={corridorsConnecting}
            connectionAttempts={corridorAttempts}
            onReconnect={() => {
              reconnectCorridors();
              reconnectAnchors();
            }}
          />
          <DataRefreshIndicator
            lastUpdated={lastUpdated}
            secondsUntilRefresh={secondsUntilRefresh}
            refreshIntervalSec={30}
            isRefreshing={isRefreshing}
            onRefresh={triggerRefresh}
          />
        </div>
      </div>

      {/* KPI Cards */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <MetricCard
          label="Payment Success Rate"
          value={`${data.kpi.successRate.value}%`}
          trend={data.kpi.successRate.trend}
          trendDirection={data.kpi.successRate.trendDirection}
        />
        <MetricCard
          label="Active Corridors"
          value={data.kpi.activeCorridors.value}
          trend={data.kpi.activeCorridors.trend}
          trendDirection={data.kpi.activeCorridors.trendDirection}
        />
        <MetricCard
          label="Liquidity Depth"
          value={formatVolume(data.kpi.liquidityDepth.value)}
          trend={data.kpi.liquidityDepth.trend}
          trendDirection={data.kpi.liquidityDepth.trendDirection}
        />
        <MetricCard
          label="Avg Settlement Speed"
          value={`${data.kpi.settlementSpeed.value}s`}
          trend={Math.abs(data.kpi.settlementSpeed.trend)}
          trendDirection={data.kpi.settlementSpeed.trendDirection}
          inverse={true} // Lower is better
        />
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-12">
        <div className="lg:col-span-8 space-y-6">
          <div className="glass-card rounded-2xl p-1 transition-all duration-300 min-h-[300px] flex flex-col">
            {data.liquidity.length > 0 ? (
              <LiquidityChart data={data.liquidity} />
            ) : (
              <div className="flex-1 flex items-center justify-center text-muted-foreground font-mono text-xs uppercase tracking-widest">
                Waiting for Liquidity Data...
              </div>
            )}
          </div>
          <div className="glass-card rounded-2xl p-1 transition-all duration-300 min-h-[300px] flex flex-col">
            {data.assets.length > 0 ? (
              <TopAssetsTable assets={data.assets} />
            ) : (
              <div className="flex-1 flex items-center justify-center text-muted-foreground font-mono text-xs uppercase tracking-widest">
                Waiting for Asset Data...
              </div>
            )}
          </div>
        </div>
        <div className="lg:col-span-4 space-y-6">
          <div className="glass-card rounded-2xl p-1 transition-all duration-300 min-h-[300px] flex flex-col">
            {data.corridors.length > 0 ? (
              <CorridorHealth corridors={data.corridors} />
            ) : (
              <div className="flex-1 flex items-center justify-center text-muted-foreground font-mono text-xs uppercase tracking-widest">
                Waiting for Corridor Data...
              </div>
            )}
          </div>
          <div className="glass-card rounded-2xl p-1 transition-all duration-300 min-h-[300px] flex flex-col">
            {data.settlement.length > 0 ? (
              <SettlementSpeedChart data={data.settlement} />
            ) : (
              <div className="flex-1 flex items-center justify-center text-muted-foreground font-mono text-xs uppercase tracking-widest">
                Waiting for Settlement Data...
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
