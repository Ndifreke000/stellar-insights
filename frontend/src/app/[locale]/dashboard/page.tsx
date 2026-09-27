"use client";

import React, { useEffect, useState, useCallback } from "react";
import { useTranslations, useLocale } from "next-intl";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { CorridorHealth } from "@/components/dashboard/CorridorHealth";
import { LiquidityChart } from "@/components/dashboard/LiquidityChart";
import { TopAssetsTable } from "@/components/dashboard/TopAssetsTable";
import { SettlementSpeedChart } from "@/components/dashboard/SettlementSpeedChart";
import { WebSocketStatus } from "@/components/WebSocketStatus";
import { DataRefreshIndicator } from "@/components/DataRefreshIndicator";
import { useRealtimeCorridors, type CorridorUpdate, type HealthAlert } from "@/hooks/useRealtimeCorridors";
import { useRealtimeAnchors, type AnchorUpdate } from "@/hooks/useRealtimeAnchors";
import { useDataRefresh } from "@/hooks/useDataRefresh";
import { logger } from "@/lib/logger";
import { BookmarksDashboardWidget } from "@/components/BookmarksDashboardWidget";
import {
  Skeleton,
  SkeletonChart,
  SkeletonCorridorCard,
  SkeletonTable
} from "@/components/ui/Skeleton";
import {
  WidgetProvider,
  WidgetGrid,
  WidgetCustomizer,
  CustomiseButton,
  type WidgetDefinition,
} from "@/components/DashboardWidgets";

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
  const t = useTranslations("dashboard");
  const locale = useLocale();
  const [data, setData] = useState<DashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // ── Data refresh hook (auto-refresh every 30 s + manual trigger) ──────────
  const fetchDashboard = useCallback(async () => {
    const response = await fetch("/api/dashboard");
    if (!response.ok) throw new Error(t("failedToFetch"));
    const result = await response.json();
    setData(result);
  }, [t]);

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
  const onCorridorUpdate = useCallback((update: CorridorUpdate) => {
    logger.debug("Received corridor update:", { update: JSON.stringify(update) });
    markUpdated();
    setData((prevData) => {
      if (!prevData) return prevData;
      const updatedData = { ...prevData };
      if (update.success_rate !== undefined) {
        updatedData.kpi.successRate.value = update.success_rate;
      }
      return updatedData;
    });
  }, [markUpdated]);

  const onHealthAlert = useCallback((alert: HealthAlert) => {
    logger.debug("Health alert:", { alert: JSON.stringify(alert) });
  }, []);

  const {
    isConnected: corridorsConnected,
    isConnecting: corridorsConnecting,
    connectionAttempts: corridorAttempts,
    reconnect: reconnectCorridors,
  } = useRealtimeCorridors({
    enablePaymentStream: true,
    onCorridorUpdate,
    onHealthAlert,
  });

  const onAnchorUpdate = useCallback((update: AnchorUpdate) => {
    logger.debug("Received anchor update:", { update: JSON.stringify(update) });
    markUpdated();
  }, [markUpdated]);

  const { isConnected: anchorsConnected, reconnect: reconnectAnchors } =
    useRealtimeAnchors({
      onAnchorUpdate,
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
        if (!isNetworkError) logger.error("Dashboard API error:", err);
      } finally {
        setLoading(false);
      }
    })();
  }, [fetchDashboard]);

  const [customizerOpen, setCustomizerOpen] = useState(false);

  if (loading) {
    return (
      <div className="space-y-8">
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
          <div className="space-y-2">
            <Skeleton className="h-4 w-24" />
            <Skeleton className="h-10 w-64" />
          </div>
          <div className="flex items-center gap-2">
            <Skeleton className="h-8 w-32" />
            <Skeleton className="h-8 w-40" />
          </div>
        </div>

        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          {[...Array(4)].map((_, i) => (
            <Skeleton className="h-32 w-full rounded-2xl" key={i} />
          ))}
        </div>

        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-12">
          <div className="lg:col-span-8 space-y-6">
            <SkeletonChart height={300} />
            <SkeletonTable rows={5} />
          </div>
          <div className="lg:col-span-4 space-y-6">
            <SkeletonCorridorCard />
            <SkeletonChart height={300} />
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="px-6 py-4 glass border-red-500/50 text-red-500 font-mono text-sm uppercase tracking-widest">
          {t("error")}: {error}
        </div>
      </div>
    );
  }

  if (!data) return null;

  const formatVolume = (val: number) => {
    return new Intl.NumberFormat(locale, {
      style: "currency",
      currency: "USD",
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(val);
  };

  // ── Widget definitions (#2109) ───────────────────────────────────────────
  const WIDGET_DEFS: WidgetDefinition[] = [
    { id: "liquidity-chart", title: "Liquidity Chart", description: "24-hour liquidity flow across corridors.", colSpan: "lg:col-span-8", minH: "min-h-[300px]", defaultVisible: true },
    { id: "assets-table", title: "Top Assets", description: "Top performing assets by volume.", colSpan: "lg:col-span-8", minH: "min-h-[300px]", defaultVisible: true },
    { id: "corridor-health", title: "Corridor Health", description: "Real-time status for active corridors.", colSpan: "lg:col-span-4", minH: "min-h-[300px]", defaultVisible: true },
    { id: "settlement-speed", title: "Settlement Speed", description: "Average payment settlement times.", colSpan: "lg:col-span-4", minH: "min-h-[300px]", defaultVisible: true },
  ];

  const renderWidget = (id: string) => {
    switch (id) {
      case "liquidity-chart":
        return (
          <div className="glass-card rounded-2xl p-1 h-full transition-all duration-300 flex flex-col">
            {data.liquidity.length > 0 ? (
              <LiquidityChart data={data.liquidity} />
            ) : (
              <div className="flex-1 flex items-center justify-center text-muted-foreground font-mono text-xs uppercase tracking-widest">
        

/* … truncated 4750 chars — edit only what you need near the top … */
