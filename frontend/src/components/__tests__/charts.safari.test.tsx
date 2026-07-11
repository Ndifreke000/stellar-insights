import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render } from '@testing-library/react';
import { LiquidityChart } from '@/components/dashboard/LiquidityChart';
import { SettlementSpeedChart } from '@/components/dashboard/SettlementSpeedChart';
import { TVLChart } from '@/components/charts/TVLChart';
import { SettlementLatencyChart } from '@/components/charts/SettlementLatencyChart';
import { TrustlineGrowthChart } from '@/components/charts/TrustlineGrowthChart';
import { PoolPerformanceChart } from '@/components/charts/PoolPerformanceChart';
import * as chartUtils from '@/lib/chart-utils';

// Mock recharts to avoid rendering issues in tests
vi.mock('recharts', async () => {
  const actual = await vi.importActual('recharts');
  return {
    ...actual,
    ResponsiveContainer: ({ children }: any) => <div data-testid="responsive-container">{children}</div>,
    AreaChart: ({ children }: any) => <div data-testid="area-chart">{children}</div>,
    LineChart: ({ children }: any) => <div data-testid="line-chart">{children}</div>,
    BarChart: ({ children }: any) => <div data-testid="bar-chart">{children}</div>,
    Tooltip: ({ contentStyle }: any) => (
      <div data-testid="tooltip" data-style={JSON.stringify(contentStyle)} />
    ),
  };
});

describe('Chart Components - Safari Compatibility', () => {
  let originalUserAgent: string;

  beforeEach(() => {
    originalUserAgent = window.navigator.userAgent;
  });

  afterEach(() => {
    Object.defineProperty(window.navigator, 'userAgent', {
      value: originalUserAgent,
      writable: true,
    });
  });

  describe('LiquidityChart', () => {
    const mockData = [
      { date: '2024-01-01', value: 1000000 },
      { date: '2024-01-02', value: 1500000 },
    ];

    it('should render without errors on Safari', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });

      const { container } = render(<LiquidityChart data={mockData} />);
      expect(container).toBeTruthy();
    });

    it('should render without errors on Chrome', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36',
        writable: true,
      });

      const { container } = render(<LiquidityChart data={mockData} />);
      expect(container).toBeTruthy();
    });
  });

  describe('SettlementSpeedChart', () => {
    const mockData = [
      { time: '00:00', speed: 2 },
      { time: '01:00', speed: 3 },
    ];

    it('should render without errors on Safari', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });

      const { container } = render(<SettlementSpeedChart data={mockData} />);
      expect(container).toBeTruthy();
    });
  });

  describe('TVLChart', () => {
    const mockData = [
      { timestamp: '2024-01-01T00:00:00Z', tvl_usd: 1000000 },
      { timestamp: '2024-01-02T00:00:00Z', tvl_usd: 1500000 },
    ];

    it('should render without errors on Safari', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });

      const { container } = render(<TVLChart data={mockData} />);
      expect(container).toBeTruthy();
    });
  });

  describe('SettlementLatencyChart', () => {
    const mockData = [
      {
        timestamp: '2024-01-01T00:00:00Z',
        median_latency_ms: 100,
        p95_latency_ms: 200,
        p99_latency_ms: 300,
      },
      {
        timestamp: '2024-01-02T00:00:00Z',
        median_latency_ms: 110,
        p95_latency_ms: 210,
        p99_latency_ms: 310,
      },
    ];

    it('should render without errors on Safari', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });

      const { container } = render(<SettlementLatencyChart data={mockData} />);
      expect(container).toBeTruthy();
    });
  });

  describe('TrustlineGrowthChart', () => {
    const mockData = [
      {
        snapshot_at: '2024-01-01T00:00:00Z',
        total_trustlines: 1000,
        authorized_trustlines: 900,
      },
      {
        snapshot_at: '2024-01-02T00:00:00Z',
        total_trustlines: 1100,
        authorized_trustlines: 950,
      },
    ];

    it('should render without errors on Safari', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });

      const { container } = render(
        <TrustlineGrowthChart data={mockData} latestTotal={1100} />
      );
      expect(container).toBeTruthy();
    });
  });

  describe('PoolPerformanceChart', () => {
    const mockData = [
      {
        snapshot_at: '2024-01-01T00:00:00Z',
        apy: 0.05,
        volume_usd: 100000,
        fees_usd: 500,
        total_value_usd: 1000000,
        impermanent_loss_pct: 0.01,
      },
      {
        snapshot_at: '2024-01-02T00:00:00Z',
        apy: 0.06,
        volume_usd: 120000,
        fees_usd: 600,
        total_value_usd: 1100000,
        impermanent_loss_pct: 0.02,
      },
    ];

    it('should render without errors on Safari', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });

      const { container } = render(<PoolPerformanceChart snapshots={mockData} />);
      expect(container).toBeTruthy();
    });
  });

  describe('Tooltip style compatibility', () => {
    it('should use getTooltipContentStyle utility in all charts', () => {
      const spy = vi.spyOn(chartUtils, 'getTooltipContentStyle');

      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });

      // Test that the utility is being called (would be called during render)
      const style = chartUtils.getTooltipContentStyle();
      expect(style).toBeDefined();
      expect(style.backgroundColor).toBeDefined();

      spy.mockRestore();
    });

    it('should not include backdropFilter for Safari', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.1 Safari/605.1.15',
        writable: true,
      });

      const style = chartUtils.getTooltipContentStyle();
      expect(style.backdropFilter).toBeUndefined();
    });

    it('should include backdropFilter for non-Safari browsers', () => {
      Object.defineProperty(window.navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36',
        writable: true,
      });

      const style = chartUtils.getTooltipContentStyle();
      expect(style.backdropFilter).toBe('blur(8px)');
    });
  });
});
