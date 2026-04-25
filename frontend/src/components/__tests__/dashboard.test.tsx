import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { KpiCard } from '../dashboard/KpiCard';
import { TrendIndicator } from '../dashboard/TrendIndicator';

// ─── KpiCard ──────────────────────────────────────────────────────────────────

describe('KpiCard', () => {
  it('renders title and value', () => {
    render(<KpiCard title="Success Rate" value="94.5%" />);
    expect(screen.getByText('Success Rate')).toBeInTheDocument();
    expect(screen.getByText('94.5%')).toBeInTheDocument();
  });

  it('renders numeric value', () => {
    render(<KpiCard title="Payments" value={1234} />);
    expect(screen.getByText('1234')).toBeInTheDocument();
  });

  it('renders subtitle when provided', () => {
    render(<KpiCard title="Latency" value="350ms" subtitle="median" />);
    expect(screen.getByText('median')).toBeInTheDocument();
  });

  it('does not render subtitle when omitted', () => {
    const { queryByText } = render(<KpiCard title="Latency" value="350ms" />);
    expect(queryByText('median')).not.toBeInTheDocument();
  });

  it('applies extra className', () => {
    const { container } = render(
      <KpiCard title="T" value="V" className="custom-class" />
    );
    expect(container.firstChild).toHaveClass('custom-class');
  });
});

// ─── TrendIndicator ───────────────────────────────────────────────────────────

describe('TrendIndicator', () => {
  it('renders neutral trend with 0%', () => {
    render(<TrendIndicator trend={{ value: 0, direction: 'neutral' }} />);
    expect(screen.getByText('0%')).toBeInTheDocument();
  });

  it('renders up trend value', () => {
    render(<TrendIndicator trend={{ value: 3.5, direction: 'up', isGood: true }} />);
    expect(screen.getByText('3.5%')).toBeInTheDocument();
  });

  it('renders down trend value', () => {
    render(<TrendIndicator trend={{ value: 2.1, direction: 'down', isGood: false }} />);
    expect(screen.getByText('2.1%')).toBeInTheDocument();
  });

  it('uses absolute value for display', () => {
    render(<TrendIndicator trend={{ value: -5.0, direction: 'down', isGood: false }} />);
    expect(screen.getByText('5.0%')).toBeInTheDocument();
  });
});
