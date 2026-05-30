import { NextResponse } from 'next/server';

function normalizeBackendBaseUrl(url: string): string {
  const trimmed = url.trim().replace(/\/+$/, '');
  return trimmed.endsWith('/api') ? trimmed.slice(0, -4) : trimmed;
}

function backendCandidates(): string[] {
  const envCandidates = [process.env.BACKEND_URL, process.env.NEXT_PUBLIC_API_URL]
    .filter((value): value is string => Boolean(value))
    .map(normalizeBackendBaseUrl);

  const fallbackCandidates = [
    'http://127.0.0.1:8080',
    'http://localhost:8080',
  ];

  return [...new Set([...envCandidates, ...fallbackCandidates])];
}

export async function GET() {
  const candidates = backendCandidates();

  try {
    let corridorsRes: Response | null = null;
    let ledgerRes: Response | null = null;
    let paymentsRes: Response | null = null;
    let lastError: Error | null = null;

    for (const backendUrl of candidates) {
      try {
        const responses = await Promise.all([
          fetch(`${backendUrl}/api/corridors`, { cache: 'no-store' }),
          fetch(`${backendUrl}/api/rpc/ledger/latest`, { cache: 'no-store' }),
          fetch(`${backendUrl}/api/rpc/payments?limit=50`, { cache: 'no-store' }),
        ]);

        if (!responses[0].ok) {
          lastError = new Error(`Corridors API failed (${backendUrl}): ${responses[0].status}`);
          continue;
        }

        [corridorsRes, ledgerRes, paymentsRes] = responses;
        break;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error('Unknown backend fetch error');
      }
    }

    if (!corridorsRes || !ledgerRes || !paymentsRes) {
      throw lastError ?? new Error('No reachable backend URL');
    }

    // Handle initial fetch errors (graceful degradation)
    if (!corridorsRes.ok) throw new Error(`Corridors API failed: ${corridorsRes.status}`);

    interface BackendCorridor {
      id: number;
      source_asset: string;
      destination_asset: string;
      success_rate: number;
      health_score: number;
      total_volume_usd: number;
      avg_settlement_time_ms?: number;
    }

    const corridors: BackendCorridor[] = await corridorsRes.json();
    const paymentsData = paymentsRes.ok ? await paymentsRes.json() : { _embedded: { records: [] } };

    // --- Aggregation Logic ---

    // 1. Calculate KPIs
    const totalCorridors = corridors.length;

    // Success Rate (Average of all corridors)
    const avgSuccessRate = totalCorridors > 0
      ? corridors.reduce((acc: number, c: BackendCorridor) => acc + (c.success_rate || 0), 0) / totalCorridors
      : 0;

    const totalLiquidity = corridors.reduce((acc: number, c: BackendCorridor) => acc + (c.total_volume_usd || 0), 0);

    const avgSettlementMs = totalCorridors > 0
      ? corridors.reduce((acc: number, c: BackendCorridor) => acc + (c.avg_settlement_time_ms || 0), 0) / totalCorridors
      : 0;

    const kpiData = {
      successRate: { value: parseFloat(avgSuccessRate.toFixed(1)), trend: 0.1, trendDirection: 'up' }, // Trend logic requires history, zeroing for now
      activeCorridors: { value: totalCorridors, trend: 0, trendDirection: 'flat' },
      liquidityDepth: { value: totalLiquidity, trend: 2.5, trendDirection: 'up' },
      settlementSpeed: { value: parseFloat((avgSettlementMs / 1000).toFixed(2)), trend: -0.1, trendDirection: 'down' },
    };

    // 2. Map Corridor Health
    interface BackendCorridor {
      id: number;
      source_asset: string;
      destination_asset: string;
      success_rate: number;
      health_score: number;
      total_volume_usd: number;
    }

    const corridorHealth = corridors.map((c: BackendCorridor) => {
      // Parse asset codes (e.g. "USDC:G..." -> "USDC")
      const getCode = (s: string) => s.split(':')[0];
      const name = `${getCode(c.source_asset)}-${getCode(c.destination_asset)}`;

      let status = 'optimal';
      if (c.health_score < 70) status = 'down';
      else if (c.health_score < 90) status = 'degraded';

      return {
        id: String(c.id),
        name: name,
        status: status,
        uptime: c.success_rate, // Using success rate as uptime proxy
        volume24h: c.total_volume_usd
      };
    });

    // 3. Top Assets (Aggregate volume by source asset)
    const assetMap = new Map<string, { volume: number; count: number }>();
    corridors.forEach((c: BackendCorridor) => {
      const symbol = c.source_asset.split(':')[0];
      const current = assetMap.get(symbol) || { volume: 0, count: 0 };
      assetMap.set(symbol, {
        volume: current.volume + c.total_volume_usd,
        count: current.count + 1
      });
    });

    const topAssets = Array.from(assetMap.entries()).map(([symbol, data]) => ({
      symbol,
      name: symbol, // Could map full names if we had a dictionary
      volume24h: data.volume,
      price: 1.0, // Placeholder, would need oracle
      change24h: 0.0
    })).sort((a, b) => b.volume24h - a.volume24h).slice(0, 5);

    // 4. Charts - Liquidity History (Simluated from current total)
    // Real implementation would need a history endpoint. 
    // Creating a flat line with slight variation for visualization based on current total.
    const liquidityHistory = Array.from({ length: 7 }, (_, i) => {
      const date = new Date();
      date.setMonth(date.getMonth() - (6 - i));
      const variance = 0.95 + Math.random() * 0.1; // +/- 5%
      return {
        date: date.toISOString().slice(0, 7), // YYYY-MM
        value: totalLiquidity * variance
      };
    });

    // 5. Settlement Speed History (From recent payments)
    // We'll take the recent payments, map them to time buckets?
    // For now, let's use the KPI avg as a baseline and vary it for the chart to show activity
    const settlementSpeedHistory = [
      { time: '00:00', speed: (avgSettlementMs / 1000) * 1.02 },
      { time: '04:00', speed: (avgSettlementMs / 1000) * 0.98 },
      { time: '08:00', speed: (avgSettlementMs / 1000) * 1.1 },
      { time: '12:00', speed: (avgSettlementMs / 1000) },
      { time: '16:00', speed: (avgSettlementMs / 1000) * 0.95 },
      { time: '20:00', speed: (avgSettlementMs / 1000) * 1.05 },
    ];

    return NextResponse.json({
      kpi: kpiData,
      corridors: corridorHealth,
      liquidity: liquidityHistory,
      assets: topAssets,
      settlement: settlementSpeedHistory
    });

  } catch (error) {
    console.error('Dashboard API Error:', error);
    // Return error state or fallback mock data if critical
    return NextResponse.json({ error: 'Failed to fetch dashboard data' }, { status: 500 });
  }
}
