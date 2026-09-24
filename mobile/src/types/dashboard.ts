export type DashboardDataSource = 'live' | 'cache' | 'mock';

export interface DashboardStats {
  volume_24h: number;
  volume_growth: number;
  avg_success_rate: number;
  success_rate_growth: number;
  active_corridors: number;
  corridors_growth: number;
}

export interface CorridorPerformance {
  corridor: string;
  success_rate: number;
  volume: number;
  health: number;
}

export interface DashboardData {
  stats: DashboardStats;
  corridor_performance: CorridorPerformance[];
}
