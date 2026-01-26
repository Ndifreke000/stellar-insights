import { NextResponse } from 'next/server'

// Mocked dashboard data — replace with RPC-driven data in the future
export async function GET() {
  const now = Date.now()
  const timeseries = Array.from({ length: 24 }).map((_, i) => ({
    ts: new Date(now - (23 - i) * 3600 * 1000).toISOString(),
    successRate: 85 + Math.round(Math.sin(i / 3) * 3),
    settlementMs: 400 + Math.round(Math.cos(i / 2) * 80),
    tvl: 1_000_000 + i * 15000 + Math.round(Math.sin(i) * 20000),
  }))

  const payload = {
    totalSuccessRate: 0.8825,
    activeCorridors: [
      { id: 'USD-EUR', health: 0.98, successRate: 0.985 },
      { id: 'XLM-USD', health: 0.92, successRate: 0.93 },
      { id: 'USDC-GBP', health: 0.89, successRate: 0.9 },
    ],
    topAssets: [
      { asset: 'USDC', volume: 12_345_678, tvl: 4_200_000 },
      { asset: 'EURT', volume: 5_123_456, tvl: 1_000_000 },
      { asset: 'XLM', volume: 3_210_000, tvl: 600_000 },
    ],
    timeseries,
  }

  return NextResponse.json(payload)
}
