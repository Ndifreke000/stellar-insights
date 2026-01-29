"use client"

import React, { useEffect, useState } from 'react';
import { MetricCard } from '@/components/dashboard/MetricCard';
import { CorridorHealth } from '@/components/dashboard/CorridorHealth';
import { LiquidityChart } from '@/components/dashboard/LiquidityChart';
import { TopAssetsTable } from '@/components/dashboard/TopAssetsTable';
import { SettlementSpeedChart } from '@/components/dashboard/SettlementSpeedChart';

interface DashboardData {
  kpi: {
    successRate: { value: number; trend: number; trendDirection: 'up' | 'down' };
    activeCorridors: { value: number; trend: number; trendDirection: 'up' | 'down' };
    liquidityDepth: { value: number; trend: number; trendDirection: 'up' | 'down' };
    settlementSpeed: { value: number; trend: number; trendDirection: 'up' | 'down' };
  };
  corridors: any[];
  liquidity: any[];
  assets: any[];
  settlement: any[];
}

export default function DashboardPage() {
  const [data, setData] = useState<DashboardData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const response = await fetch('/api/dashboard');
        if (!response.ok) {
          throw new Error('Failed to fetch dashboard data');
        }
        const result = await response.json();
        setData(result);
      } catch (err) {
        // Only log non-network errors to avoid noise
        const isNetworkError = err instanceof TypeError && 
          (err.message.includes('Failed to fetch') || 
           err.message.includes('fetch is not defined') ||
           err.message.includes('Network request failed'));
           
        const errorMessage = err instanceof Error ? err.message : 'An error occurred';
        setError(errorMessage);
        
        if (!isNetworkError) {
          console.error("Dashboard API error:", err);
        }
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, []);

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="text-lg text-muted-foreground animate-pulse">Loading dashboard insights...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="text-lg text-red-500">Error: {error}</div>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className="flex-1 space-y-4 p-8 pt-6">
      <div className="flex items-center justify-between space-y-2">
        <h2 className="text-3xl font-bold tracking-tight">Network Overview</h2>
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
          value={`$${(data.kpi.liquidityDepth.value / 1000000).toFixed(1)}M`}
          trend={data.kpi.liquidityDepth.trend}
          trendDirection={data.kpi.liquidityDepth.trendDirection}
        />
        <MetricCard
          label="Avg Settlement Speed"
          value={`${data.kpi.settlementSpeed.value}s`}
          trend={Math.abs(data.kpi.settlementSpeed.trend)}
          trendDirection={data.kpi.settlementSpeed.trendDirection}
        />
      </div>

      {/* Charts Row 1 */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
        <div className="col-span-4 transition-all duration-300 hover:shadow-md">
          <LiquidityChart data={data.liquidity} />
        </div>
        <div className="col-span-3 transition-all duration-300 hover:shadow-md">
          <CorridorHealth corridors={data.corridors} />
        </div>
      </div>

      {/* Charts Row 2 */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
        <div className="col-span-3 transition-all duration-300 hover:shadow-md">
          <SettlementSpeedChart data={data.settlement} />
        </div>
        <div className="col-span-4 transition-all duration-300 hover:shadow-md">
          <TopAssetsTable assets={data.assets} />
        </div>
      </div>
    </div>
  );
}
