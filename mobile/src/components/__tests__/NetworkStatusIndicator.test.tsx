import React from 'react';
import { render } from '@testing-library/react-native';
import { NetworkStatusIndicator } from '../NetworkStatusIndicator';
import { useNetworkStatusIndicator } from '@hooks/useNetworkStatusIndicator';

jest.mock('@hooks/useNetworkStatusIndicator', () => ({
  useNetworkStatusIndicator: jest.fn(),
}));

const mockedUseNetworkStatusIndicator = useNetworkStatusIndicator as jest.MockedFunction<typeof useNetworkStatusIndicator>;

describe('NetworkStatusIndicator', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockedUseNetworkStatusIndicator.mockReturnValue({
      status: 'offline',
      message: 'Offline mode active',
      isVisible: true,
      isOnline: false,
      isSyncing: false,
      platformOffset: 0,
      dismiss: jest.fn(),
      show: jest.fn(),
    });
  });

  it('renders correctly', async () => {
    const { getByText } = await render(<NetworkStatusIndicator />);

    expect(getByText('Network Status Indicator')).toBeTruthy();
    expect(getByText('Offline mode active')).toBeTruthy();
  });

  it('renders nothing when hidden', async () => {
    mockedUseNetworkStatusIndicator.mockReturnValue({
      status: 'online',
      message: 'Back online',
      isVisible: false,
      isOnline: true,
      isSyncing: false,
      platformOffset: 0,
      dismiss: jest.fn(),
      show: jest.fn(),
    });

    const { toJSON } = await render(<NetworkStatusIndicator />);

    expect(toJSON()).toBeNull();
  });
});
