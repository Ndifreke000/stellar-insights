import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';

import { AnchorDetail } from '@components/AnchorDetail';
import type { AnchorDetailData } from '@app-types/anchor';

const mockGoBack = jest.fn();
const mockGetParent = jest.fn();
const mockParentNavigate = jest.fn();
const mockRefetch = jest.fn();
const mockUseAnchorDetail = jest.fn();

jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({
    goBack: mockGoBack,
    getParent: mockGetParent,
  }),
  useRoute: () => ({
    params: { anchorId: 'anchor-1' },
  }),
}));

jest.mock('@hooks/useAnchorDetail', () => ({
  useAnchorDetail: (...args: unknown[]) => mockUseAnchorDetail(...args),
}));

function createMockAnchorData(anchorId: string): AnchorDetailData {
  return {
    anchor: {
      id: anchorId,
      name: 'MoneyGram Access',
      stellar_account: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
      reliability_score: 99.2,
      asset_coverage: 2,
      failure_rate: 0.8,
      total_transactions: 12450,
      successful_transactions: 12350,
      failed_transactions: 100,
      status: 'green',
    },
    issued_assets: [
      {
        asset_code: 'USDC',
        issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
        volume_24h_usd: 1_250_000,
        success_rate: 98.5,
        failure_rate: 1.5,
        total_transactions: 5400,
      },
    ],
    reliability_history: [{ timestamp: '2026-05-01', score: 95 }],
    top_failure_reasons: [{ reason: 'Timeout awaiting response', count: 45 }],
    recent_failed_corridors: [
      {
        corridor_id: 'USDC-PHP',
        timestamp: new Date().toISOString(),
      },
    ],
  };
}

function mockHookReturn(overrides: Record<string, unknown> = {}) {
  return {
    data: createMockAnchorData('anchor-1'),
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

describe('Anchor Detail', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockGetParent.mockReturnValue({
      navigate: mockParentNavigate,
    });
  });

  it('renders correctly', () => {
    mockUseAnchorDetail.mockReturnValue(mockHookReturn());

    const { getByText } = render(<AnchorDetail anchorId="anchor-1" />);

    expect(getByText('MoneyGram Access')).toBeTruthy();
    expect(getByText('Issued Assets')).toBeTruthy();
  });

  it('shows loading state with accessibility label', () => {
    mockUseAnchorDetail.mockReturnValue(
      mockHookReturn({ data: null, loading: true }),
    );

    const { getByLabelText } = render(<AnchorDetail anchorId="anchor-1" />);

    expect(getByLabelText('Loading anchor detail')).toBeTruthy();
  });

  it('shows error state and retries on button press', async () => {
    mockUseAnchorDetail.mockReturnValue(
      mockHookReturn({
        data: null,
        error: 'Failed to load anchor data',
      }),
    );

    const { getByLabelText } = render(<AnchorDetail anchorId="anchor-1" />);

    fireEvent.press(getByLabelText('Retry loading anchor detail'));
    await waitFor(() => {
      expect(mockRefetch).toHaveBeenCalledTimes(1);
    });
  });

  it('shows mock sample-data banner when live data is unavailable', () => {
    mockUseAnchorDetail.mockReturnValue(
      mockHookReturn({
        dataSource: 'mock',
        warning: 'Live data unavailable. Showing sample data.',
      }),
    );

    const { getByText } = render(<AnchorDetail anchorId="anchor-1" />);

    expect(getByText('Live data unavailable. Showing sample data.')).toBeTruthy();
    expect(
      getByText(
        'Metrics shown are for preview only and may not reflect live anchor performance.',
      ),
    ).toBeTruthy();
  });

  it('shows offline cached feedback banner', () => {
    mockUseAnchorDetail.mockReturnValue(
      mockHookReturn({
        isOffline: true,
        dataSource: 'cache',
        isFromCache: true,
        warning: 'Offline — showing saved anchor data.',
      }),
    );

    const { getByText } = render(<AnchorDetail anchorId="anchor-1" />);

    expect(getByText('Offline — showing saved anchor data.')).toBeTruthy();
  });

  it('uses navigation callbacks when props are not provided', () => {
    mockUseAnchorDetail.mockReturnValue(mockHookReturn());

    const { getByLabelText } = render(<AnchorDetail anchorId="anchor-1" />);

    fireEvent.press(getByLabelText('Go back to anchors list'));
    expect(mockGoBack).toHaveBeenCalledTimes(1);

    fireEvent.press(getByLabelText('Open failed corridor USDC-PHP'));
    expect(mockParentNavigate).toHaveBeenCalledWith('Corridors', {
      screen: 'CorridorDetail',
      params: { corridorId: 'USDC-PHP' },
    });
  });

  it('uses provided navigation callbacks when supplied', () => {
    const onGoBack = jest.fn();
    const onNavigateToCorridor = jest.fn();
    mockUseAnchorDetail.mockReturnValue(mockHookReturn());

    const { getByLabelText } = render(
      <AnchorDetail
        anchorId="anchor-1"
        onGoBack={onGoBack}
        onNavigateToCorridor={onNavigateToCorridor}
      />,
    );

    fireEvent.press(getByLabelText('Go back to anchors list'));
    fireEvent.press(getByLabelText('Open failed corridor USDC-PHP'));

    expect(onGoBack).toHaveBeenCalledTimes(1);
    expect(onNavigateToCorridor).toHaveBeenCalledWith('USDC-PHP');
    expect(mockGoBack).not.toHaveBeenCalled();
  });

  it('shows default error message when data is unavailable', () => {
    mockUseAnchorDetail.mockReturnValue(
      mockHookReturn({ data: null, error: null }),
    );

    const { getByText } = render(<AnchorDetail anchorId="anchor-1" />);
    expect(getByText('Failed to load anchor data')).toBeTruthy();
  });

  it('covers reliability score color branches', () => {
    const warningData = createMockAnchorData('anchor-1');
    warningData.anchor.reliability_score = 80;
    mockUseAnchorDetail.mockReturnValue(mockHookReturn({ data: warningData }));

    const { getByText, rerender } = render(<AnchorDetail anchorId="anchor-1" />);
    expect(getByText('80.0')).toBeTruthy();

    const criticalData = createMockAnchorData('anchor-1');
    criticalData.anchor.reliability_score = 60;
    mockUseAnchorDetail.mockReturnValue(mockHookReturn({ data: criticalData }));
    rerender(<AnchorDetail anchorId="anchor-1" />);
    expect(getByText('60.0')).toBeTruthy();
  });

  it('covers status style branches', () => {
    const statuses = ['yellow', 'red', 'unknown'] as const;

    statuses.forEach(status => {
      const data = createMockAnchorData('anchor-1');
      data.anchor.status = status;
      mockUseAnchorDetail.mockReturnValue(mockHookReturn({ data }));
      const { getByText } = render(<AnchorDetail anchorId="anchor-1" />);
      if (status === 'yellow') {
        expect(getByText('Degraded')).toBeTruthy();
      } else if (status === 'red') {
        expect(getByText('Critical')).toBeTruthy();
      } else {
        expect(getByText('Unknown')).toBeTruthy();
      }
    });
  });

  it('refetches when pull-to-refresh is triggered', () => {
    mockUseAnchorDetail.mockReturnValue(mockHookReturn());

    const { UNSAFE_getByType } = render(<AnchorDetail anchorId="anchor-1" />);
    const scrollView = UNSAFE_getByType(require('react-native').ScrollView);
    scrollView.props.refreshControl.props.onRefresh();

    expect(mockRefetch).toHaveBeenCalledTimes(1);
  });

  it('renders short addresses and varied asset volume formats', () => {
    const data = createMockAnchorData('anchor-1');
    data.anchor.stellar_account = 'GSHORT';
    data.anchor.total_transactions = 0;
    data.anchor.successful_transactions = 0;
    data.issued_assets = [
      {
        asset_code: 'XLM',
        issuer: 'GSHORT',
        volume_24h_usd: 500,
        success_rate: 90,
        failure_rate: 10,
        total_transactions: 10,
      },
      {
        asset_code: 'BTC',
        issuer: 'GSHORT',
        volume_24h_usd: 500_000,
        success_rate: 80,
        failure_rate: 20,
        total_transactions: 5,
      },
    ];
    mockUseAnchorDetail.mockReturnValue(mockHookReturn({ data }));

    const { getByLabelText, getByText, getAllByText } = render(
      <AnchorDetail anchorId="anchor-1" />,
    );

    expect(getAllByText('GSHORT').length).toBeGreaterThan(0);
    expect(getByLabelText('Uptime 0.0 percent')).toBeTruthy();
    expect(getByText('Uptime')).toBeTruthy();
  });

  it('shows cache feedback banner styling for non-mock data source', () => {
    mockUseAnchorDetail.mockReturnValue(
      mockHookReturn({
        dataSource: 'cache',
        warning: 'Offline — showing saved anchor data.',
      }),
    );

    const { getByText } = render(<AnchorDetail anchorId="anchor-1" />);
    expect(getByText('Offline — showing saved anchor data.')).toBeTruthy();
  });

  it('covers additional status aliases', () => {
    const aliases = [
      { status: 'active', label: 'Healthy' },
      { status: 'healthy', label: 'Healthy' },
      { status: 'warning', label: 'Degraded' },
      { status: 'inactive', label: 'Critical' },
      { status: 'critical', label: 'Critical' },
    ] as const;

    aliases.forEach(({ status, label }) => {
      const data = createMockAnchorData('anchor-1');
      data.anchor.status = status;
      mockUseAnchorDetail.mockReturnValue(mockHookReturn({ data }));
      const { getByText } = render(<AnchorDetail anchorId="anchor-1" />);
      expect(getByText(label)).toBeTruthy();
    });
  });

  it('omits optional sections when data is not provided', () => {
    const data = createMockAnchorData('anchor-1');
    data.issued_assets = [];
    data.top_failure_reasons = [];
    data.recent_failed_corridors = [];
    mockUseAnchorDetail.mockReturnValue(mockHookReturn({ data }));

    const { queryByText } = render(<AnchorDetail anchorId="anchor-1" />);

    expect(queryByText('Issued Assets')).toBeNull();
    expect(queryByText('Top Failure Reasons')).toBeNull();
    expect(queryByText('Recent Failed Corridors')).toBeNull();
  });

  it('does not navigate to corridor when parent navigator is unavailable', () => {
    mockGetParent.mockReturnValue(null);
    mockUseAnchorDetail.mockReturnValue(mockHookReturn());

    const { getByLabelText } = render(<AnchorDetail anchorId="anchor-1" />);
    fireEvent.press(getByLabelText('Open failed corridor USDC-PHP'));

    expect(mockParentNavigate).not.toHaveBeenCalled();
  });
});
