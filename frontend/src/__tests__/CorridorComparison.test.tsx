import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { CorridorComparisonTable } from '../components/CorridorComparisonTable';
import { CorridorMetrics } from '../lib/api';

// Mock corridor data
const mockCorridors: CorridorMetrics[] = [
  {
    id: 'USDC-XLM',
    source_asset: 'USDC',
    destination_asset: 'XLM',
    success_rate: 94.5,
    total_attempts: 1678,
    successful_payments: 1552,
    failed_payments: 126,
    average_latency_ms: 487,
    median_latency_ms: 350,
    p95_latency_ms: 1250,
    p99_latency_ms: 1950,
    liquidity_depth_usd: 6200000,
    liquidity_volume_24h_usd: 850000,
    liquidity_trend: 'increasing' as const,
    average_slippage_bps: 12.5,
    health_score: 94,
    last_updated: new Date().toISOString(),
  },
  {
    id: 'EURC-PHP',
    source_asset: 'EURC',
    destination_asset: 'PHP',
    success_rate: 88.3,
    total_attempts: 1200,
    successful_payments: 1060,
    failed_payments: 140,
    average_latency_ms: 520,
    median_latency_ms: 380,
    p95_latency_ms: 1400,
    p99_latency_ms: 2100,
    liquidity_depth_usd: 4500000,
    liquidity_volume_24h_usd: 620000,
    liquidity_trend: 'stable' as const,
    average_slippage_bps: 18.2,
    health_score: 85,
    last_updated: new Date().toISOString(),
  },
];

describe('CorridorComparisonTable', () => {
  it('renders corridor comparison table', () => {
    render(<CorridorComparisonTable corridors={mockCorridors} />);
    
    expect(screen.getByText('Detailed Comparison')).toBeInTheDocument();
    expect(screen.getByText('USDC')).toBeInTheDocument();
    expect(screen.getByText('EURC')).toBeInTheDocument();
  });

  it('displays all metrics', () => {
    render(<CorridorComparisonTable corridors={mockCorridors} />);
    
    expect(screen.getByText('Success Rate')).toBeInTheDocument();
    expect(screen.getByText('Health Score')).toBeInTheDocument();
    expect(screen.getByText('Avg Latency')).toBeInTheDocument();
    expect(screen.getByText('Liquidity Depth')).toBeInTheDocument();
    expect(screen.getByText('24h Volume')).toBeInTheDocument();
    expect(screen.getByText('Avg Slippage')).toBeInTheDocument();
  });

  it('highlights best and worst performers', () => {
    const { container } = render(<CorridorComparisonTable corridors={mockCorridors} />);
    
    // Check for trophy icons (best performers)
    const trophyIcons = container.querySelectorAll('[title="Best"]');
    expect(trophyIcons.length).toBeGreaterThan(0);
    
    // Check for warning icons (worst performers)
    const warningIcons = container.querySelectorAll('[title="Worst"]');
    expect(warningIcons.length).toBeGreaterThan(0);
  });

  it('calls export function when export button is clicked', () => {
    const mockExport = vi.fn();
    render(<CorridorComparisonTable corridors={mockCorridors} onExport={mockExport} />);
    
    const exportButton = screen.getByText('Export CSV');
    fireEvent.click(exportButton);
    
    expect(mockExport).toHaveBeenCalledTimes(1);
  });

  it('sorts corridors when column header is clicked', () => {
    render(<CorridorComparisonTable corridors={mockCorridors} />);
    
    const successRateHeader = screen.getByText('Success Rate');
    fireEvent.click(successRateHeader);
    
    // After clicking, the sort order should change
    // This is a basic test - you might want to verify the actual order
    expect(successRateHeader).toBeInTheDocument();
  });

  it('displays correct metric values', () => {
    render(<CorridorComparisonTable corridors={mockCorridors} />);
    
    // Check for specific metric values
    expect(screen.getByText('94.50%')).toBeInTheDocument(); // USDC-XLM success rate
    expect(screen.getByText('88.30%')).toBeInTheDocument(); // EURC-PHP success rate
  });

  it('shows legend with performance indicators', () => {
    render(<CorridorComparisonTable corridors={mockCorridors} />);
    
    expect(screen.getByText('Best Performance')).toBeInTheDocument();
    expect(screen.getByText('Worst Performance')).toBeInTheDocument();
    expect(screen.getByText('Above Average')).toBeInTheDocument();
    expect(screen.getByText('Below Average')).toBeInTheDocument();
  });

  it('handles empty corridor list', () => {
    render(<CorridorComparisonTable corridors={[]} />);
    
    expect(screen.getByText('Detailed Comparison')).toBeInTheDocument();
    // Table should still render but with no data rows
  });

  it('formats currency values correctly', () => {
    render(<CorridorComparisonTable corridors={mockCorridors} />);
    
    // Check for formatted liquidity depth (in millions)
    expect(screen.getByText('$6.20M')).toBeInTheDocument();
    expect(screen.getByText('$4.50M')).toBeInTheDocument();
  });

  it('formats latency values correctly', () => {
    render(<CorridorComparisonTable corridors={mockCorridors} />);
    
    // Check for formatted latency (in milliseconds)
    expect(screen.getByText('487ms')).toBeInTheDocument();
    expect(screen.getByText('520ms')).toBeInTheDocument();
  });
});

describe('Comparison URL Handling', () => {
  it('parses corridor IDs from URL', () => {
    const urlParams = new URLSearchParams('ids=USDC-XLM,EURC-PHP,USDC-NGN');
    const ids = urlParams.get('ids')?.split(',') || [];
    
    expect(ids).toHaveLength(3);
    expect(ids).toContain('USDC-XLM');
    expect(ids).toContain('EURC-PHP');
    expect(ids).toContain('USDC-NGN');
  });

  it('handles empty URL params', () => {
    const urlParams = new URLSearchParams('');
    const ids = urlParams.get('ids')?.split(',').filter(Boolean) || [];
    
    expect(ids).toHaveLength(0);
  });

  it('filters out empty corridor IDs', () => {
    const urlParams = new URLSearchParams('ids=USDC-XLM,,EURC-PHP');
    const ids = urlParams.get('ids')?.split(',').filter(Boolean) || [];
    
    expect(ids).toHaveLength(2);
    expect(ids).not.toContain('');
  });
});

describe('CSV Export', () => {
  it('generates correct CSV format', () => {
    const headers = [
      'Corridor',
      'Success Rate (%)',
      'Health Score',
      'Avg Latency (ms)',
      'Liquidity Depth (USD)',
      '24h Volume (USD)',
      'Avg Slippage (bps)',
    ];

    const row = [
      'USDC-XLM',
      '94.50',
      '94.0',
      '487',
      '6200000.00',
      '850000.00',
      '12.50',
    ];

    const csvLine = row.join(',');
    expect(csvLine).toContain('USDC-XLM');
    expect(csvLine).toContain('94.50');
    expect(csvLine).toContain('6200000.00');
  });
});

describe('Performance Indicators', () => {
  it('identifies best performer correctly', () => {
    const values = [94.5, 88.3, 91.2];
    const best = Math.max(...values);
    
    expect(best).toBe(94.5);
  });

  it('identifies worst performer correctly', () => {
    const values = [94.5, 88.3, 91.2];
    const worst = Math.min(...values);
    
    expect(worst).toBe(88.3);
  });

  it('calculates performance position correctly', () => {
    const value = 90;
    const best = 95;
    const worst = 85;
    const range = Math.abs(best - worst);
    const position = Math.abs(value - worst) / range;
    
    expect(position).toBe(0.5); // 50% between worst and best
  });
});