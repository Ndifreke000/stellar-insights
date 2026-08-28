import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';

import { CorridorDetail } from '@components/CorridorDetail';
import type { CorridorDetailData } from '@app-types/corridor';

const mockGoBack = jest.fn();
const mockPush = jest.fn();
const mockRefetch = jest.fn();
const mockUseCorridorDetail = jest.fn();

jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({
    goBack: mockGoBack,
    push: mockPush,
  }),
  useRoute: () => ({
    params: { corridorId: 'USDC-PHP' },
  }),
}));

jest.mock('@hooks/useCorridorDetail', () => ({
  useCorridorDetail: (...args: unknown[]) => mockUseCorridorDetail(...args),
}));

function createMockCorridorData(corridorId: string): CorridorDetailData {
  return {
    corridor: {
      id: corridorId,
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
    historical_success_rate: [{ timestamp: '2026-05-01', success_rate: 90, attempts: 100 }],
    latency_distribution: [{ latency_bucket_ms: 100, count: 10, percentage: 100 }],
    liquidity_trends: [{ timestamp: '2026-05-01', liquidity_usd: 1000, volume_24h_usd: 500 }],
    historical_volume: [{ timestamp: '2026-05-01', volume_usd: 1000 }],
    historical_slippage: [{ timestamp: '2026-05-01', average_slippage_bps: 10 }],
    related_corridors: [
      {
        id: 'corridor-2',
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
    ],
  };
}

function mockHookReturn(overrides: Record<string, unknown> = {}) {
  return {
    data: createMockCorridorData('USDC-PHP'),
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

describe('Corridor Detail', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders correctly', () => {
    mockUseCorridorDetail.mockReturnValue(mockHookReturn());

    const { getByText } = render(<CorridorDetail corridorId="USDC-PHP" />);

    expect(getByText('USDC → PHP')).toBeTruthy();
    expect(getByText('Pair: USDC-PHP')).toBeTruthy();
  });

  it('shows loading state with accessibility label', () => {
    mockUseCorridorDetail.mockReturnValue(
      mockHookReturn({ data: null, loading: true }),
    );

    const { getByLabelText } = render(<CorridorDetail corridorId="USDC-PHP" />);

    expect(getByLabelText('Loading corridor detail')).toBeTruthy();
  });

  it('shows error state and retries on button press', async () => {
    mockUseCorridorDetail.mockReturnValue(
      mockHookReturn({
        data: null,
        error: 'Failed to load corridor data',
      }),
    );

    const { getByLabelText } = render(<CorridorDetail corridorId="USDC-PHP" />);

    fireEvent.press(getByLabelText('Retry loading corridor detail'));
    await waitFor(() => {
      expect(mockRefetch).toHaveBeenCalledTimes(1);
    });
  });

  it('shows mock sample-data banner when live data is unavailable', () => {
    mockUseCorridorDetail.mockReturnValue(
      mockHookReturn({
        dataSource: 'mock',
        warning: 'Live data unavailable. Showing sample data.',
      }),
    );

    const { getByText } = render(<CorridorDetail corridorId="USDC-PHP" />);

    expect(getByText('Live data unavailable. Showing sample data.')).toBeTruthy();
    expect(
      getByText(
        'Metrics shown are for preview only and may not reflect live corridor performance.',
      ),
    ).toBeTruthy();
  });

  it('shows offline cached feedback banner', () => {
    mockUseCorridorDetail.mockReturnValue(
      mockHookReturn({
        isOffline: true,
        dataSource: 'cache',
        isFromCache: true,
        warning: 'Offline — showing saved corridor data.',
      }),
    );

    const { getByText } = render(<CorridorDetail corridorId="USDC-PHP" />);

    expect(getByText('Offline — showing saved corridor data.')).toBeTruthy();
  });

  it('uses navigation callbacks when props are not provided', () => {
    mockUseCorridorDetail.mockReturnValue(mockHookReturn());

    const { getByLabelText } = render(<CorridorDetail corridorId="USDC-PHP" />);

    fireEvent.press(getByLabelText('Go back to corridors list'));
    expect(mockGoBack).toHaveBeenCalledTimes(1);

    fireEvent.press(getByLabelText('Open corridor USDC to JPY'));
    expect(mockPush).toHaveBeenCalledWith('CorridorDetail', {
      corridorId: 'corridor-2',
    });
  });

  it('uses provided navigation callbacks when supplied', () => {
    const onGoBack = jest.fn();
    const onNavigateToCorridor = jest.fn();
    mockUseCorridorDetail.mockReturnValue(mockHookReturn());

    const { getByLabelText } = render(
      <CorridorDetail
        corridorId="USDC-PHP"
        onGoBack={onGoBack}
        onNavigateToCorridor={onNavigateToCorridor}
      />,
    );

    fireEvent.press(getByLabelText('Go back to corridors list'));
    fireEvent.press(getByLabelText('Open corridor USDC to JPY'));

    expect(onGoBack).toHaveBeenCalledTimes(1);
    expect(onNavigateToCorridor).toHaveBeenCalledWith('corridor-2');
    expect(mockGoBack).not.toHaveBeenCalled();
  });

  it('shows default error message when data is unavailable', () => {
    mockUseCorridorDetail.mockReturnValue(
      mockHookReturn({ data: null, error: null }),
    );

    const { getByText } = render(<CorridorDetail corridorId="USDC-PHP" />);
    expect(getByText('Failed to load corridor data')).toBeTruthy();
  });

  it('covers low and warning health score branches', () => {
    const warningData = createMockCorridorData('USDC-PHP');
    warningData.corridor.health_score = 80;
    mockUseCorridorDetail.mockReturnValue(mockHookReturn({ data: warningData }));

    const { getByText, rerender } = render(
      <CorridorDetail corridorId="USDC-PHP" />,
    );
    expect(getByText('80.0')).toBeTruthy();

    const criticalData = createMockCorridorData('USDC-PHP');
    criticalData.corridor.health_score = 60;
    mockUseCorridorDetail.mockReturnValue(mockHookReturn({ data: criticalData }));
    rerender(<CorridorDetail corridorId="USDC-PHP" />);
    expect(getByText('60.0')).toBeTruthy();
  });

  it('refetches when pull-to-refresh is triggered', () => {
    mockUseCorridorDetail.mockReturnValue(mockHookReturn());

    const { UNSAFE_getByType } = render(<CorridorDetail corridorId="USDC-PHP" />);
    const scrollView = UNSAFE_getByType(require('react-native').ScrollView);
    scrollView.props.refreshControl.props.onRefresh();

    expect(mockRefetch).toHaveBeenCalledTimes(1);
  });
});
