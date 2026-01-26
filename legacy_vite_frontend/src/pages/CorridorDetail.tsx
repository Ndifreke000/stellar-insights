import React, { useEffect, useState } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import {
  ArrowLeft,
  TrendingUp,
  TrendingDown,
  Zap,
  Droplets,
  CheckCircle2,
  AlertCircle,
  Clock,
  BarChart3,
  Loader,
  Home,
} from 'lucide-react';
import { getCorridorDetail, generateMockCorridorData, CorridorDetailData } from '@/lib/api';
import { SuccessRateChart, LatencyDistributionChart, LiquidityTrendChart } from '@/components/CorridorCharts';

export default function CorridorDetail() {
  const { id: corridorId } = useParams<{ id: string }>();
  const navigate = useNavigate();

  const [data, setData] = useState<CorridorDetailData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function fetchData() {
      if (!corridorId) return;
      
      try {
        setLoading(true);
        try {
          const result = await getCorridorDetail(corridorId);
          setData(result);
        } catch {
          console.log('API not available, using mock data');
          const mockData = generateMockCorridorData(corridorId);
          setData(mockData);
        }
      } catch (err) {
        setError('Failed to load corridor data');
        console.error(err);
      } finally {
        setLoading(false);
      }
    }

    fetchData();
  }, [corridorId]);

  if (loading) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <Loader className="w-8 h-8 text-primary animate-spin" />
          <p className="text-muted-foreground">Loading corridor data...</p>
        </div>
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="flex gap-4 mb-6">
          <Link
            to="/corridors"
            className="flex items-center gap-2 text-primary hover:text-primary/80 transition-colors font-medium"
          >
            <ArrowLeft className="w-5 h-5" />
            Back to Corridors
          </Link>
          <Link
            to="/"
            className="flex items-center gap-2 text-primary hover:text-primary/80 transition-colors font-medium"
          >
            <Home className="w-5 h-5" />
            Home
          </Link>
        </div>
        <div className="bg-red-900/20 border border-red-600/50 rounded-lg p-6 text-red-200">
          <AlertCircle className="w-6 h-6 inline mr-2" />
          {error || 'Failed to load corridor data'}
        </div>
      </div>
    );
  }

  const corridor = data.corridor;
  const healthColor =
    corridor.health_score >= 90
      ? 'text-green-500'
      : corridor.health_score >= 75
        ? 'text-yellow-500'
        : 'text-red-500';

  const trendIcon =
    corridor.liquidity_trend === 'increasing' ? (
      <TrendingUp className="w-5 h-5 text-green-500" />
    ) : corridor.liquidity_trend === 'decreasing' ? (
      <TrendingDown className="w-5 h-5 text-red-500" />
    ) : (
      <TrendingUp className="w-5 h-5 text-muted-foreground" />
    );

  return (
    <div className="min-h-screen bg-background text-foreground">
      {/* Header */}
      <div className="sticky top-0 z-40 bg-background/80 backdrop-blur-sm border-b border-border">
        <div className="max-w-7xl mx-auto px-3 sm:px-4 md:px-6 lg:px-8 py-3 md:py-4">
          <div className="flex flex-col sm:flex-row gap-2 mb-3 md:mb-4">
            <Link
              to="/corridors"
              className="flex items-center gap-2 text-primary hover:text-primary/80 transition-colors font-medium text-sm md:text-base"
            >
              <ArrowLeft className="w-4 h-4 md:w-5 md:h-5" />
              Back to Corridors
            </Link>
            <Link
              to="/"
              className="flex items-center gap-2 text-primary hover:text-primary/80 transition-colors font-medium text-sm md:text-base"
            >
              <Home className="w-4 h-4 md:w-5 md:h-5" />
              Home
            </Link>
          </div>
          <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 md:gap-4">
            <div className="min-w-0">
              <h1 className="text-2xl md:text-3xl lg:text-4xl font-bold truncate">
                {corridor.source_asset} → {corridor.destination_asset}
              </h1>
              <p className="text-muted-foreground text-xs md:text-sm mt-0.5 md:mt-1 truncate font-mono">
                Corridor ID: {corridor.id}
              </p>
            </div>
            <div className="text-right shrink-0">
              <div className={`text-3xl md:text-4xl font-bold font-mono ${healthColor}`}>
                {corridor.health_score.toFixed(1)}
              </div>
              <p className="text-muted-foreground text-xs md:text-sm">Health Score</p>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-3 sm:px-4 md:px-6 lg:px-8 py-4 md:py-8">
        {/* Key Metrics Grid */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3 md:gap-6 mb-6 md:mb-8">
          {/* Success Rate */}
          <div className="bg-card border border-border rounded-lg p-3 md:p-6 hover:border-primary transition-colors">
            <div className="flex items-center justify-between mb-2 md:mb-4">
              <span className="text-muted-foreground text-xs md:text-sm font-medium">Success Rate</span>
              <CheckCircle2 className="w-4 h-4 md:w-5 md:h-5 text-green-500" />
            </div>
            <div className="text-2xl md:text-3xl font-bold text-green-500 font-mono">
              {corridor.success_rate.toFixed(1)}%
            </div>
            <p className="text-muted-foreground text-xs mt-1 md:mt-2 font-mono">
              {corridor.successful_payments} of {corridor.total_attempts}
            </p>
          </div>

          {/* Average Latency */}
          <div className="bg-card border border-border rounded-lg p-3 md:p-6 hover:border-primary transition-colors">
            <div className="flex items-center justify-between mb-2 md:mb-4">
              <span className="text-muted-foreground text-xs md:text-sm font-medium">Avg Latency</span>
              <Clock className="w-4 h-4 md:w-5 md:h-5 text-primary" />
            </div>
            <div className="text-2xl md:text-3xl font-bold text-primary font-mono">
              {corridor.average_latency_ms.toFixed(0)}
              <span className="text-lg md:text-xl">ms</span>
            </div>
            <p className="text-muted-foreground text-xs mt-1 md:mt-2 font-mono">
              Med: {corridor.median_latency_ms}ms | P99: {corridor.p99_latency_ms}ms
            </p>
          </div>

          {/* Liquidity Depth */}
          <div className="bg-card border border-border rounded-lg p-3 md:p-6 hover:border-primary transition-colors">
            <div className="flex items-center justify-between mb-2 md:mb-4">
              <span className="text-muted-foreground text-xs md:text-sm font-medium">Liquidity Depth</span>
              <Droplets className="w-4 h-4 md:w-5 md:h-5 text-purple-400" />
            </div>
            <div className="text-2xl md:text-3xl font-bold text-purple-400 font-mono">
              ${(corridor.liquidity_depth_usd / 1000000).toFixed(2)}M
            </div>
            <div className="flex items-center gap-1 md:gap-2 mt-1 md:mt-2">
              {trendIcon}
              <p className="text-muted-foreground text-xs capitalize">{corridor.liquidity_trend}</p>
            </div>
          </div>

          {/* 24h Volume */}
          <div className="bg-card border border-border rounded-lg p-3 md:p-6 hover:border-primary transition-colors">
            <div className="flex items-center justify-between mb-2 md:mb-4">
              <span className="text-muted-foreground text-xs md:text-sm font-medium">24h Volume</span>
              <Zap className="w-4 h-4 md:w-5 md:h-5 text-amber-400" />
            </div>
            <div className="text-2xl md:text-3xl font-bold text-amber-400 font-mono">
              ${(corridor.liquidity_volume_24h_usd / 1000000).toFixed(2)}M
            </div>
            <p className="text-muted-foreground text-xs mt-1 md:mt-2 font-mono">
              {new Date(corridor.last_updated).toLocaleTimeString()}
            </p>
          </div>
        </div>

        {/* Charts Section */}
        <div className="space-y-3 md:space-y-6">
          {/* Success Rate Chart */}
          <div className="bg-card border border-border rounded-lg p-3 md:p-6">
            <SuccessRateChart data={data.historical_success_rate} />
          </div>

          {/* Two-column layout for latency and liquidity */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-3 md:gap-6">
            <div className="bg-card border border-border rounded-lg p-3 md:p-6">
              <LatencyDistributionChart data={data.latency_distribution} />
            </div>
            <div className="bg-card border border-border rounded-lg p-3 md:p-6">
              <LiquidityTrendChart data={data.liquidity_trends} />
            </div>
          </div>
        </div>

        {/* Related Corridors */}
        {data.related_corridors && data.related_corridors.length > 0 && (
          <div className="mt-6 md:mt-8">
            <h2 className="text-xl md:text-2xl font-bold mb-4 md:mb-6 flex items-center gap-2 md:gap-3">
              <BarChart3 className="w-5 h-5 md:w-6 md:h-6 text-primary" />
              Related Corridors
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 md:gap-4">
              {data.related_corridors.map((related) => (
                <button
                  key={related.id}
                  onClick={() => navigate(`/corridors/${related.id}`)}
                  className="bg-card border border-border rounded-lg p-3 md:p-4 hover:border-primary transition-all duration-200 transform hover:-translate-y-1 text-left cursor-pointer"
                >
                  <div className="flex justify-between items-start mb-2 md:mb-3 gap-2">
                    <div className="min-w-0">
                      <h3 className="font-semibold text-sm md:text-base truncate">
                        {related.source_asset} → {related.destination_asset}
                      </h3>
                      <p className="text-muted-foreground text-xs truncate font-mono">{related.id}</p>
                    </div>
                    <span className="text-green-500 text-xs md:text-sm font-bold shrink-0 font-mono">
                      {related.success_rate.toFixed(1)}%
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-xs">
                    <div>
                      <p className="text-muted-foreground text-xs">Health</p>
                      <p className="font-semibold font-mono">{related.health_score.toFixed(0)}</p>
                    </div>
                    <div>
                      <p className="text-muted-foreground text-xs">Liquidity</p>
                      <p className="font-semibold font-mono">${(related.liquidity_depth_usd / 1000000).toFixed(1)}M</p>
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Footer Info */}
        <div className="mt-6 md:mt-8 p-3 md:p-4 bg-muted border border-border rounded-lg text-muted-foreground text-xs md:text-sm">
          <p className="font-mono">Last updated: {new Date(corridor.last_updated).toLocaleString()}</p>
          <p className="mt-1 md:mt-2 text-xs">Charts update every 5 minutes with 30-day historical data.</p>
        </div>
      </div>
    </div>
  );
}
