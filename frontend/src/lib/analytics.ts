import { api, ApiError } from "./api";

/**
 * Analytics Data Interfaces
 */

export interface NetworkVolumeDataPoint {
    time: string;
    volume: number;
    corridors: number;
    anchors: number;
}

export interface CorridorPerformanceMetric {
    corridor: string;
    successRate: number;
    volume: number;
    health: number;
}

export interface NetworkStats {
    volume24h: number;
    volumeGrowth: number;
    avgSuccessRate: number;
    successRateGrowth: number;
    activeCorridors: number;
    corridorsGrowth: number;
}

export interface AnalyticsDashboardData {
    stats: NetworkStats;
    timeSeriesData: NetworkVolumeDataPoint[];
    corridorPerformance: CorridorPerformanceMetric[];
}

/**
 * Mock Data Generator
 */
function generateMockAnalyticsData(): AnalyticsDashboardData {
    // Mock data for charts
    const timeSeriesData: NetworkVolumeDataPoint[] = [
        { time: "00:00", volume: 45000, corridors: 18, anchors: 42 },
        { time: "04:00", volume: 52000, corridors: 21, anchors: 45 },
        { time: "08:00", volume: 48000, corridors: 19, anchors: 48 },
        { time: "12:00", volume: 61000, corridors: 24, anchors: 52 },
        { time: "16:00", volume: 55000, corridors: 22, anchors: 50 },
        { time: "20:00", volume: 67000, corridors: 25, anchors: 56 },
        { time: "23:59", volume: 72000, corridors: 28, anchors: 62 },
    ];

    const corridorPerformance: CorridorPerformanceMetric[] = [
        { corridor: "USDC→PHP", successRate: 98.5, volume: 240000, health: 95 },
        { corridor: "USD→PHP", successRate: 97.2, volume: 180000, health: 92 },
        { corridor: "EUR→USDC", successRate: 99.1, volume: 150000, health: 98 },
        { corridor: "USDC→SGD", successRate: 96.8, volume: 120000, health: 89 },
        { corridor: "USD→EUR", successRate: 98.9, volume: 200000, health: 97 },
    ];

    const stats: NetworkStats = {
        volume24h: 2400000,
        volumeGrowth: 18,
        avgSuccessRate: 98.1,
        successRateGrowth: 0.8,
        activeCorridors: 24,
        corridorsGrowth: 3,
    };

    return {
        stats,
        timeSeriesData,
        corridorPerformance,
    };
}

/**
 * Fetch Analytics Dashboard Data
 */
export async function getAnalyticsDashboard(): Promise<AnalyticsDashboardData> {
    // TODO: Replace with actual API call when backend is ready
    // return api.get<AnalyticsDashboardData>('/analytics/dashboard');

    return new Promise((resolve) => {
        setTimeout(() => {
            resolve(generateMockAnalyticsData());
        }, 1000); // Simulate network latency
    });
}
