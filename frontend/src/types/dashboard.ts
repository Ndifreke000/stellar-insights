// Dashboard Types for Type Safety

export interface KPIMetric {
  value: number;
  trend: number;
  trendDirection: 'up' | 'down';
}

export interface DashboardKPI {
  successRate: KPIMetric;
  activeCorridors: KPIMetric;
  liquidityDepth: KPIMetric;
  settlementSpeed: KPIMetric;
}

export interface CorridorHealth {
  id: string;
  name: string;
  status: 'optimal' | 'degraded' | 'critical';
  uptime: number;
  volume24h: number;
}

export interface LiquidityDataPoint {
  date: string;
  value: number;
}

export interface AssetData {
  symbol: string;
  name: string;
  volume24h: number;
  price: number;
  change24h: number;
}

export interface SettlementSpeedDataPoint {
  time: string;
  speed: number;
}

export interface DashboardData {
  kpi: DashboardKPI;
  corridors: CorridorHealth[];
  liquidity: LiquidityDataPoint[];
  assets: AssetData[];
  settlement: SettlementSpeedDataPoint[];
}

// Validation functions
export function validateCorridorHealth(corridor: any): corridor is CorridorHealth {
  return (
    typeof corridor.id === 'string' &&
    typeof corridor.name === 'string' &&
    ['optimal', 'degraded', 'critical'].includes(corridor.status) &&
    typeof corridor.uptime === 'number' &&
    typeof corridor.volume24h === 'number'
  );
}

export function validateLiquidityDataPoint(point: any): point is LiquidityDataPoint {
  return (
    typeof point.date === 'string' &&
    typeof point.value === 'number'
  );
}

export function validateAssetData(asset: any): asset is AssetData {
  return (
    typeof asset.symbol === 'string' &&
    typeof asset.name === 'string' &&
    typeof asset.volume24h === 'number' &&
    typeof asset.price === 'number' &&
    typeof asset.change24h === 'number'
  );
}

export function validateSettlementSpeedDataPoint(point: any): point is SettlementSpeedDataPoint {
  return (
    typeof point.time === 'string' &&
    typeof point.speed === 'number'
  );
}