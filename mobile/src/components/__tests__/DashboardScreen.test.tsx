import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';

import { DashboardScreen } from '@components/DashboardScreen';
import type { CorridorPerformance, DashboardStats } from '@app-types/dashboard';

const mockRefetch = jest.fn();
const mockUseDashboardScreen = jest.fn();

jest.mock('@hooks/useDashboardScreen', () => ({
  useDashboardScreen: (...args: unknown[]) => mockUseDashboardScreen(...args),
}));

const SAMPLE_STATS: DashboardStats = {
  volume_24h: 72000,
  volume_growth: 8.4,
  avg_success_rate: 96.1,
  success_rate_growth: 1.2,
  active_corridors: 28,
  corridors_growth: 3,
};

const SAMPLE_CORRIDORS: CorridorPerformance[] = [
  { corridor: 'USDC→PHP', success_rate: 98.5, volume: 240000, health: 95 },
  { corridor: 'EUR→USDC', success_rate: 99.1, volume: 150000, health: 98 },
  { corridor: 'USDC→SGD', success_rate: 96.8, volume: 120000, health: 89 },
];

function mockHookReturn(overrides: Record<string, unknown> = {}) {
  return {
    stats: SAMPLE_STATS,
    corridorPerformance: SAMPLE_CORRIDORS,
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

describe('Dashboard Screen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders correctly', () => {
    mockUseDashboardScreen.mockReturnValue(mockHookReturn());

    const { getByText } = render(<DashboardScreen />);

    expect(getByText('Dashboard')).toBeTruthy();
    expect(getByText('Top Corridors')).toBeTruthy();
    expect(getByText('USDC→PHP')).toBeTruthy();
  });

  it('renders stat cards with formatted values', () => {
    mockUseDashboardScreen.mockReturnValue(mockHookReturn());

    const { getByText } = render(<DashboardScreen />);

    expect(getByText('Volume (24h)')).toBeTruthy();
    expect(getByText('$72.0K')).toBeTruthy();
    expect(getByText('Success Rate')).toBeTruthy();
    expect(getByText('96.1%')).toBeTruthy();
    expect(getByText('Active Corridors')).toBeTruthy();
    expect(getByText('28')).toBeTruthy();
  });

  it('shows loading state with accessibility label', () => {
    mockUseDashboardScreen.mockReturnValue(
      mockHookReturn({ stats: null, corridorPerformance: [], loading: true }),
    );

    const { getByLabelText } = render(<DashboardScreen />);

    expect(getByLabelText('Loading dashboard')).toBeTruthy();
  });

  it('shows error state and retries on button press', async () => {
    mockUseDashboardScreen.mockReturnValue(
      mockHookReturn({
        stats: null,
        corridorPerformance: [],
        error: 'Failed to load dashboard',
      }),
    );

    const { getByLabelText } = render(<DashboardScreen />);

    fireEvent.press(getByLabelText('Retry loading dashboard'));
    await waitFor(() => {
      expect(mockRefetch).toHaveBeenCalledTimes(1);
    });
  });

  it('shows mock sample-data banner when live data is unavailable', () => {
    mockUseDashboardScreen.mockReturnValue(
      mockHookReturn({
        dataSource: 'mock',
        warning: 'Live data unavailable. Showing sample dashboard.',
      }),
    );

    const { getByText } = render(<DashboardScreen />);

    expect(
      getByText('Live data unavailable. Showing sample dashboard.'),
    ).toBeTruthy();
    expect(
      getByText(
        'Dashboard metrics shown are for preview only and may not reflect live performance.',
      ),
    ).toBeTruthy();
  });

  it('shows offline cached feedback banner', () => {
    mockUseDashboardScreen.mockReturnValue(
      mockHookReturn({
        isOffline: true,
        dataSource: 'cache',
        isFromCache: true,
        warning: 'Offline — showing saved dashboard.',
      }),
    );

    const { getByText } = render(<DashboardScreen />);

    expect(getByText('Offline — showing saved dashboard.')).toBeTruthy();
  });

  it('invokes onCorridorPress when a corridor row is pressed', () => {
    mockUseDashboardScreen.mockReturnValue(mockHookReturn());
    const onCorridorPress = jest.fn();

    const { getByText } = render(
      <DashboardScreen onCorridorPress={onCorridorPress} />,
    );

    fireEvent.press(getByText('USDC→PHP'));
    expect(onCorridorPress).toHaveBeenCalledWith('USDC→PHP');
  });

  it('formats large and small values and negative/neutral growth', () => {
    mockUseDashboardScreen.mockReturnValue(
      mockHookReturn({
        stats: {
          volume_24h: 2_400_000,
          volume_growth: -5.3,
          avg_success_rate: 88.0,
          success_rate_growth: 0,
          active_corridors: 12,
          corridors_growth: 0,
        },
        corridorPerformance: [
          // Fair health (75-89) and a sub-$1K volume.
          { corridor: 'USD→EUR', success_rate: 80.0, volume: 850, health: 80 },
          // Critical health (<75).
          { corridor: 'EURC→NGN', success_rate: 70.0, volume: 2100, health: 60 },
        ],
      }),
    );

    const { getByText, getByLabelText } = render(<DashboardScreen />);

    expect(getByText('$2.4M')).toBeTruthy();
    expect(getByText('-5.3%')).toBeTruthy();
    expect(getByLabelText(/USD→EUR, Fair/)).toBeTruthy();
    expect(getByLabelText(/EURC→NGN, Critical/)).toBeTruthy();
    expect(getByLabelText(/Success Rate.*no change/)).toBeTruthy();
  });

  it('shows an empty state when no corridor data is available', () => {
    mockUseDashboardScreen.mockReturnValue(
      mockHookReturn({ corridorPerformance: [] }),
    );

    const { getByText } = render(<DashboardScreen />);

    expect(getByText('No corridor data available')).toBeTruthy();
  });
});
