/**
 * Mock Monitoring Data Service
 * Provides realistic data for the monitoring dashboard charts.
 */

export interface MonitoringStats {
  pageLoadTimes: { timestamp: string; value: number }[];
  apiLatencies: { endpoint: string; latency: number }[];
  errorRates: { timestamp: string; count: number }[];
  sessionStats: {
    totalSessions: number;
    avgDuration: number;
    bounceRate: number;
  };
  browserBreakdown: { name: string; value: number }[];
  deviceBreakdown: { name: string; value: number }[];
}

export const getMockMonitoringStats = (): MonitoringStats => {
  const now = new Date();
  const points = 24;

  // Page load times (last 24 hours)
  const pageLoadTimes = Array.from({ length: points }).map((_, i) => ({
    timestamp: new Date(
      now.getTime() - (points - i) * 3600000,
    ).toLocaleTimeString([], { hour: "2-digit" }),
    value: 1.2 + Math.random() * 0.8,
  }));

  // API Latencies
  const apiLatencies = [
    { endpoint: "/api/anchors", latency: 320 },
    { endpoint: "/api/corridors", latency: 450 },
    { endpoint: "/api/transactions", latency: 890 },
    { endpoint: "/api/health", latency: 120 },
    { endpoint: "/api/metrics", latency: 210 },
  ];

  // Error rates
  const errorRates = Array.from({ length: points }).map((_, i) => ({
    timestamp: new Date(
      now.getTime() - (points - i) * 3600000,
    ).toLocaleTimeString([], { hour: "2-digit" }),
    count: Math.floor(Math.random() * 5),
  }));

  // Session stats
  const sessionStats = {
    totalSessions: 12450,
    avgDuration: 345, // seconds
    bounceRate: 24.5,
  };

  // Browser breakdown
  const browserBreakdown = [
    { name: "Chrome", value: 65 },
    { name: "Safari", value: 20 },
    { name: "Firefox", value: 10 },
    { name: "Edge", value: 5 },
  ];

  // Device breakdown
  const deviceBreakdown = [
    { name: "Desktop", value: 70 },
    { name: "Mobile", value: 25 },
    { name: "Tablet", value: 5 },
  ];

  return {
    pageLoadTimes,
    apiLatencies,
    errorRates,
    sessionStats,
    browserBreakdown,
    deviceBreakdown,
  };
};
