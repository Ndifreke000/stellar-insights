import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';

import { CorridorsList } from '@components/CorridorsList';
import type { CorridorMetrics } from '@types/corridor';

const mockRefetch = jest.fn();
const mockUseCorridorsList = jest.fn();
const mockNavigate = jest.fn();

jest.mock('@hooks/useCorridorsList', () => ({
  useCorridorsList: (...args: unknown[]) => mockUseCorridorsList(...args),
}));

jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({
    navigate: mockNavigate,
  }),
}));

const SAMPLE_CORRIDORS: CorridorMetrics[] = [
  {
    id: 'USDC-PHP',
    source_asset: 'USDC',
    destination_asset: 'PHP',
    success_rate: 92.5,
    total_attempts: 1678,
    successful_payments: 1552,
    failed_payments: 126,
    average_latency_ms: 487,
    median_latency_ms: 350,
    p95_latency_ms: 1250,
    p99_latency_ms: 1950,
    liquidity_depth_usd: 6200000,
    liquidity_volume_24h_usd: 850000,
    liquidity_trend: 'increasing',
    average_slippage_bps: 12.5,
    health_score: 94,
    last_updated: new Date().toISOString(),
  },
  {
    id: 'USDC-JPY',
    source_asset: 'USDC',
    destination_asset: 'JPY',
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
    liquidity_trend: 'stable',
    average_slippage_bps: 18.2,
    health_score: 85,
    last_updated: new Date().toISOString(),
  },
  {
    id: 'EURC-NGN',
    source_asset: 'EURC',
    destination_asset: 'NGN',
    success_rate: 76.4,
    total_attempts: 890,
    successful_payments: 680,
    failed_payments: 210,
    average_latency_ms: 720,
    median_latency_ms: 540,
    p95_latency_ms: 1800,
    p99_latency_ms: 2600,
    liquidity_depth_usd: 2100000,
    liquidity_volume_24h_usd: 310000,
    liquidity_trend: 'decreasing',
    average_slippage_bps: 24.8,
    health_score: 68,
    last_updated: new Date().toISOString(),
  },
  {
    id: 'USDC-EUR',
    source_asset: 'USDC',
    destination_asset: 'EUR',
    success_rate: 95.0,
    total_attempts: 500,
    successful_payments: 475,
    failed_payments: 25,
    average_latency_ms: 300,
    median_latency_ms: 250,
    p95_latency_ms: 800,
    p99_latency_ms: 1200,
    liquidity_depth_usd: 1500,
    liquidity_volume_24h_usd: 900,
    liquidity_trend: 'stable',
    average_slippage_bps: 8.0,
    health_score: 72,
    last_updated: new Date().toISOString(),
  },
];

function mockHookReturn(overrides: Record<string, unknown> = {}) {
  return {
    corridors: SAMPLE_CORRIDORS.slice(0, 3),
    total: 3,
    loading: false,
    error: null,
    warning: null,
    isOffline: false,
    dataSource: 'live',
    isFromCache: false,
    refetch: mockRefetch,
    ...overrides,
  };
}

describe('Corridors List', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders correctly', () => {
    mockUseCorridorsList.mockReturnValue(mockHookReturn());

    const { getByText } = render(<CorridorsList />);

    expect(getByText('Corridors List')).toBeTruthy();
    expect(getByText('USDC → PHP')).toBeTruthy();
    expect(getByText('USDC → JPY')).toBeTruthy();
  });

  it('shows loading state with accessibility label', () => {
    mockUseCorridorsList.mockReturnValue(
      mockHookReturn({ corridors: [], total: 0, loading: true }),
    );

    const { getByLabelText } = render(<CorridorsList />);

    expect(getByLabelText('Loading corridors list')).toBeTruthy();
  });

  it('shows error state and retries on button press', async () => {
    mockUseCorridorsList.mockReturnValue(
      mockHookReturn({
        corridors: [],
        total: 0,
        error: 'Failed to load corridors',
      }),
    );

    const { getByLabelText } = render(<CorridorsList />);

    fireEvent.press(getByLabelText('Retry loading corridors list'));
    await waitFor(() => {
      expect(mockRefetch).toHaveBeenCalledTimes(1);
    });
  });

  it('shows mock sample-data banner when live data is unavailable', () => {
    mockUseCorridorsList.mockReturnValue(
      mockHookReturn({
        dataSource: 'mock',
        warning: 'Live data unavailable. Showing sample corridors.',
      }),
    );

    const { getByText } = render(<CorridorsList />);

    expect(getByText('Live data unavailable. Showing sample corridors.')).toBeTruthy();
    expect(
      getByText(
        'Corridor metrics shown are for preview only and may not reflect live performance.',
      ),
    ).toBeTruthy();
  });

  it('shows offline cached feedback banner', () => {
    mockUseCorridorsList.mockReturnValue(
      mockHookReturn({
        isOffline: true,
        dataSource: 'cache',
        isFromCache: true,
        warning: 'Offline — showing saved corridors.',
      }),
    );

    const { getByText } = render(<CorridorsList />);

    expect(getByText('Offline — showing saved corridors.')).toBeTruthy();
  });

  it('shows empty state when no corridors are available', () => {
    mockUseCorridorsList.mockReturnValue(
      mockHookReturn({ corridors: [], total: 0 }),
    );

    const { getByLabelText } = render(<CorridorsList />);

    expect(getByLabelText('No corridors available')).toBeTruthy();
  });

  it('calls onCorridorPress when a corridor card is pressed', () => {
    const onCorridorPress = jest.fn();
    mockUseCorridorsList.mockReturnValue(mockHookReturn());

    const { getByLabelText } = render(
      <CorridorsList onCorridorPress={onCorridorPress} />,
    );

    fireEvent.press(
      getByLabelText(
        'USDC to PHP, Healthy, success rate 92.5 percent, health score 94',
      ),
    );

    expect(onCorridorPress).toHaveBeenCalledWith('USDC-PHP');
  });

  it('navigates to corridor detail when no onCorridorPress is provided', () => {
    mockUseCorridorsList.mockReturnValue(mockHookReturn());

    const { getByLabelText } = render(<CorridorsList />);

    fireEvent.press(
      getByLabelText(
        'USDC to PHP, Healthy, success rate 92.5 percent, health score 94',
      ),
    );

    expect(mockNavigate).toHaveBeenCalledWith('CorridorDetail', {
      corridorId: 'USDC-PHP',
    });
  });

  it('refetches when pull-to-refresh is triggered', () => {
    mockUseCorridorsList.mockReturnValue(mockHookReturn());

    const { UNSAFE_getByType } = render(<CorridorsList />);
    const flatList = UNSAFE_getByType(require('react-native').FlatList);
    flatList.props.refreshControl.props.onRefresh();

    expect(mockRefetch).toHaveBeenCalledTimes(1);
  });

  it('renders health badges for healthy, fair, and critical corridors', () => {
    mockUseCorridorsList.mockReturnValue(
      mockHookReturn({ corridors: SAMPLE_CORRIDORS.slice(0, 3), total: 3 }),
    );

    const { getByText } = render(<CorridorsList />);

    expect(getByText('Healthy')).toBeTruthy();
    expect(getByText('Fair')).toBeTruthy();
    expect(getByText('Critical')).toBeTruthy();
  });

  it('formats liquidity values below one thousand', () => {
    mockUseCorridorsList.mockReturnValue(
      mockHookReturn({
        corridors: [{ ...SAMPLE_CORRIDORS[3], liquidity_depth_usd: 750 }],
        total: 1,
      }),
    );

    const { getByText } = render(<CorridorsList />);

    expect(getByText('$750')).toBeTruthy();
  });

  it('formats liquidity values in millions and thousands', () => {
    mockUseCorridorsList.mockReturnValue(
      mockHookReturn({
        corridors: [SAMPLE_CORRIDORS[0], { ...SAMPLE_CORRIDORS[3], liquidity_depth_usd: 1500 }],
        total: 2,
      }),
    );

    const { getByText } = render(<CorridorsList />);

    expect(getByText('$6.2M')).toBeTruthy();
    expect(getByText('$1.5K')).toBeTruthy();
  });
});
