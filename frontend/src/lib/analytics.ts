
/**
 * Analytics Data Interfaces
 */

import { api } from './api/api';

export interface NetworkVolumeDataPoint {
    time: string;
    volume: number;
    corridors: number;
    anchors: number;
}

export interface CorridorPerformanceMetric {
    corridor: string;
    success_rate: number;
    volume: number;
    health: number;
}

export interface NetworkStats {
    volume_24h: number;
    volume_growth: number;
    avg_success_rate: number;
    success_rate_growth: number;
    active_corridors: number;
    corridors_growth: number;
}

export interface AnalyticsDashboardData {
    stats: NetworkStats;
    time_series_data: NetworkVolumeDataPoint[];
    corridor_performance: CorridorPerformanceMetric[];
}

/**
 * Mock Data Generator
 */
function generateMockAnalyticsData(): AnalyticsDashboardData {
    // Mock data for charts
    const time_series_data: NetworkVolumeDataPoint[] = [
        { time: "00:00", volume: 45000, corridors: 18, anchors: 42 },
        { time: "04:00", volume: 52000, corridors: 21, anchors: 45 },
        { time: "08:00", volume: 48000, corridors: 19, anchors: 48 },
        { time: "12:00", volume: 61000, corridors: 24, anchors: 52 },
        { time: "16:00", volume: 55000, corridors: 22, anchors: 50 },
        { time: "20:00", volume: 67000, corridors: 25, anchors: 56 },
        { time: "23:59", volume: 72000, corridors: 28, anchors: 62 },
    ];

    const corridor_performance: CorridorPerformanceMetric[] = [
        { corridor: "USDC→PHP", success_rate: 98.5, volume: 240000, health: 95 },
        { corridor: "USD→PHP", success_rate: 97.2, volume: 180000, health: 92 },
        { corridor: "EUR→USDC", success_rate: 99.1, volume: 150000, health: 98 },
        { corridor: "USDC→SGD", success_rate: 96.8, volume: 120000, health: 89 },
        { corridor: "USD→EUR", success_rate: 98.9, volume: 200000, health: 97 },
    ];

    const stats: NetworkStats = {
        volume_24h: 2400000,
        volume_growth: 18,
        avg_success_rate: 98.1,
        success_rate_growth: 0.8,
        active_corridors: 24,
        corridors_growth: 3,
    };

    return {
        stats,
        time_series_data,
        corridor_performance,
    };
}

/**
 * Fetch Analytics Dashboard Data
 */
export async function getAnalyticsDashboard(): Promise<AnalyticsDashboardData> {
    try {
        const response = await api.get<AnalyticsDashboardData>('/analytics/dashboard');
        return response;
    } catch (error) {
        // Fallback to mock data if backend is unavailable
        console.warn('Backend analytics endpoint unavailable, using mock data:', error);
        return new Promise((resolve) => {
            setTimeout(() => {
                resolve(generateMockAnalyticsData());
            }, 1000); // Simulate network latency
        });
    }
}
