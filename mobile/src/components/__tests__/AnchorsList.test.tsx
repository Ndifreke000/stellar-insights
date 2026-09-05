import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';

import { AnchorsList } from '@components/AnchorsList';
import type { AnchorListItem } from '@hooks/useAnchorsList';

const mockRefetch = jest.fn();
const mockUseAnchorsList = jest.fn();
const mockNavigate = jest.fn();

jest.mock('@hooks/useAnchorsList', () => ({
  useAnchorsList: (...args: unknown[]) => mockUseAnchorsList(...args),
}));

jest.mock('@react-navigation/native', () => ({
  useNavigation: () => ({
    navigate: mockNavigate,
  }),
}));

const SAMPLE_ANCHORS: AnchorListItem[] = [
  {
    id: 'anchor-1',
    name: 'MoneyGram Access',
    stellar_account: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    reliability_score: 99.2,
    asset_coverage: 5,
    failure_rate: 0.8,
    total_transactions: 12450,
    successful_transactions: 12350,
    failed_transactions: 100,
    status: 'green',
  },
  {
    id: 'anchor-2',
    name: 'AnchorUSD',
    stellar_account: 'GBBD47IF6LWK7P7MDEVSCWR7DPUXV3NAM7KNR4WTXV7X5FDT5O6ADFOY',
    reliability_score: 91.5,
    asset_coverage: 3,
    failure_rate: 8.5,
    total_transactions: 8320,
    successful_transactions: 7610,
    failed_transactions: 710,
    status: 'yellow',
  },
  {
    id: 'anchor-3',
    name: 'Demo Anchor',
    stellar_account: 'GCKFBEIYTKPGAQQLRGSTNATTJHUUOWD63AANJPLUYFXXQWSK3PXMKYC7',
    reliability_score: 72.4,
    asset_coverage: 2,
    failure_rate: 27.6,
    total_transactions: 2100,
    successful_transactions: 1520,
    failed_transactions: 580,
    status: 'red',
  },
  {
    id: 'anchor-4',
    name: 'Legacy Anchor',
    stellar_account: 'GSHORT',
    reliability_score: 80,
    asset_coverage: 1,
    failure_rate: 5,
    total_transactions: 0,
    successful_transactions: 0,
    failed_transactions: 0,
    status: 'unknown',
  },
];

function mockHookReturn(overrides: Record<string, unknown> = {}) {
  return {
    anchors: SAMPLE_ANCHORS.slice(0, 3),
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

describe('Anchors List', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders correctly', async () => {
    mockUseAnchorsList.mockReturnValue(mockHookReturn());

    const { getByText } = await render(<AnchorsList />);

    expect(getByText('Anchors List')).toBeTruthy();
    expect(getByText('MoneyGram Access')).toBeTruthy();
    expect(getByText('AnchorUSD')).toBeTruthy();
  });

  it('shows loading state with accessibility label', async () => {
    mockUseAnchorsList.mockReturnValue(
      mockHookReturn({ anchors: [], total: 0, loading: true }),
    );

    const { getByLabelText } = await render(<AnchorsList />);

    expect(getByLabelText('Loading anchors list')).toBeTruthy();
  });

  it('shows error state and retries on button press', async () => {
    mockUseAnchorsList.mockReturnValue(
      mockHookReturn({
        anchors: [],
        total: 0,
        error: 'Failed to load anchors',
      }),
    );

    const { getByLabelText } = await render(<AnchorsList />);

    await fireEvent.press(getByLabelText('Retry loading anchors list'));
    await waitFor(() => {
      expect(mockRefetch).toHaveBeenCalledTimes(1);
    });
  });

  it('shows mock sample-data banner when live data is unavailable', async () => {
    mockUseAnchorsList.mockReturnValue(
      mockHookReturn({
        dataSource: 'mock',
        warning: 'Live data unavailable. Showing sample anchors.',
      }),
    );

    const { getByText } = await render(<AnchorsList />);

    expect(getByText('Live data unavailable. Showing sample anchors.')).toBeTruthy();
    expect(
      getByText(
        'Anchor metrics shown are for preview only and may not reflect live performance.',
      ),
    ).toBeTruthy();
  });

  it('shows offline cached feedback banner', async () => {
    mockUseAnchorsList.mockReturnValue(
      mockHookReturn({
        isOffline: true,
        dataSource: 'cache',
        isFromCache: true,
        warning: 'Offline — showing saved anchors.',
      }),
    );

    const { getByText } = await render(<AnchorsList />);

    expect(getByText('Offline — showing saved anchors.')).toBeTruthy();
  });

  it('shows empty state when no anchors are available', async () => {
    mockUseAnchorsList.mockReturnValue(
      mockHookReturn({ anchors: [], total: 0 }),
    );

    const { getByLabelText } = await render(<AnchorsList />);

    expect(getByLabelText('No anchors available')).toBeTruthy();
  });

  it('calls onAnchorPress when an anchor card is pressed', async () => {
    const onAnchorPress = jest.fn();
    mockUseAnchorsList.mockReturnValue(mockHookReturn());

    const { getByLabelText } = await render(
      <AnchorsList onAnchorPress={onAnchorPress} />,
    );

    await fireEvent.press(
      getByLabelText(
        'MoneyGram Access, Healthy, reliability 99.2 percent, uptime 99.2 percent',
      ),
    );

    expect(onAnchorPress).toHaveBeenCalledWith('anchor-1');
  });

  it('navigates to anchor detail when no onAnchorPress is provided', async () => {
    mockUseAnchorsList.mockReturnValue(mockHookReturn());

    const { getByLabelText } = await render(<AnchorsList />);

    await fireEvent.press(
      getByLabelText(
        'MoneyGram Access, Healthy, reliability 99.2 percent, uptime 99.2 percent',
      ),
    );

    expect(mockNavigate).toHaveBeenCalledWith('AnchorDetail', {
      anchorId: 'anchor-1',
    });
  });

  it('refetches when pull-to-refresh is triggered', async () => {
    mockUseAnchorsList.mockReturnValue(mockHookReturn());

    const { getByTestId } = await render(<AnchorsList />);
    const flatList = getByTestId('anchors-list');
    flatList.props.refreshControl.props.onRefresh();

    expect(mockRefetch).toHaveBeenCalledTimes(1);
  });

  it('renders status badges for healthy, degraded, critical, and unknown anchors', async () => {
    mockUseAnchorsList.mockReturnValue(
      mockHookReturn({ anchors: SAMPLE_ANCHORS, total: 4 }),
    );

    const { getByText } = await render(<AnchorsList />);

    expect(getByText('Healthy')).toBeTruthy();
    expect(getByText('Degraded')).toBeTruthy();
    expect(getByText('Critical')).toBeTruthy();
    expect(getByText('Unknown')).toBeTruthy();
  });

  it('formats large transaction counts and short addresses', async () => {
    mockUseAnchorsList.mockReturnValue(
      mockHookReturn({
        anchors: [
          {
            ...SAMPLE_ANCHORS[3],
            total_transactions: 2_500_000,
            successful_transactions: 2_400_000,
          },
        ],
        total: 1,
      }),
    );

    const { getByText } = await render(<AnchorsList />);

    expect(getByText('GSHORT')).toBeTruthy();
    expect(getByText('2.4M / 2.5M transactions · Failure rate 5.0%')).toBeTruthy();
  });

  it('formats thousands in transaction counts', async () => {
    mockUseAnchorsList.mockReturnValue(
      mockHookReturn({
        anchors: [
          {
            ...SAMPLE_ANCHORS[0],
            total_transactions: 5500,
            successful_transactions: 4200,
          },
        ],
        total: 1,
      }),
    );

    const { getByText } = await render(<AnchorsList />);

    expect(getByText('4.2K / 5.5K transactions · Failure rate 0.8%')).toBeTruthy();
  });
});
