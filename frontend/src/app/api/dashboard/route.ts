import { NextResponse } from 'next/server';

export async function GET() {
  // Mock Data
  const kpiData = {
    successRate: { value: 99.8, trend: 0.2, trendDirection: 'up' },
    activeCorridors: { value: 12, trend: 1, trendDirection: 'up' },
    liquidityDepth: { value: 45000000, trend: 5.5, trendDirection: 'up' }, // In USD
    settlementSpeed: { value: 3.2, trend: -0.5, trendDirection: 'down' }, // In seconds
  };

  const corridorHealth = [
    { id: '1', name: 'BRZ-NGN', status: 'optimal', uptime: 99.9, volume24h: 1200000 },
    { id: '2', name: 'USD-EUR', status: 'optimal', uptime: 100, volume24h: 5400000 },
    { id: '3', name: 'EUR-NGN', status: 'degraded', uptime: 95.5, volume24h: 800000 },
    { id: '4', name: 'USD-ARS', status: 'optimal', uptime: 99.2, volume24h: 2100000 },
  ];

  const liquidityHistory = [
    { date: '2023-01', value: 30000000 },
    { date: '2023-02', value: 32000000 },
    { date: '2023-03', value: 31000000 },
    { date: '2023-04', value: 35000000 },
    { date: '2023-05', value: 38000000 },
    { date: '2023-06', value: 42000000 },
    { date: '2023-07', value: 45000000 },
  ];

  const topAssets = [
    { symbol: 'USDC', name: 'USD Coin', volume24h: 12000000, price: 1.00, change24h: 0.01 },
    { symbol: 'XLM', name: 'Stellar Lumens', volume24h: 8500000, price: 0.11, change24h: 2.5 },
    { symbol: 'EURT', name: 'Euro', volume24h: 3200000, price: 1.09, change24h: -0.1 },
    { symbol: 'NGNC', name: 'Nigerian Naira', volume24h: 1500000, price: 0.00064, change24h: 0.0 },
  ];

  const settlementSpeedHistory = [
      { time: '00:00', speed: 3.1 },
      { time: '04:00', speed: 3.5 },
      { time: '08:00', speed: 2.8 },
      { time: '12:00', speed: 3.2 },
      { time: '16:00', speed: 4.0 },
      { time: '20:00', speed: 3.0 },
  ];


  return NextResponse.json({
    kpi: kpiData,
    corridors: corridorHealth,
    liquidity: liquidityHistory,
    assets: topAssets,
    settlement: settlementSpeedHistory
  });
}
