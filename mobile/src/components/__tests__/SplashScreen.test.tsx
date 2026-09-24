import React from 'react';
import { render } from '@testing-library/react-native';
import { SplashScreen } from '../SplashScreen';
import { useSplashScreen } from '@hooks/useSplashScreen';

jest.mock('@hooks/useSplashScreen', () => ({
  useSplashScreen: jest.fn(),
}));

const mockedUseSplashScreen = useSplashScreen as jest.MockedFunction<typeof useSplashScreen>;

const baseResult = {
  status: 'loading' as const,
  error: null,
  isVisible: true,
  platformName: 'iOS',
};

describe('SplashScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockedUseSplashScreen.mockReturnValue(baseResult);
  });

  it('renders correctly in loading state', async () => {
    const { getByText, getByLabelText } = await render(<SplashScreen />);

    expect(getByText('PayRaider')).toBeTruthy();
    expect(getByLabelText('Loading')).toBeTruthy();
  });

  it('shows error message in error state', async () => {
    mockedUseSplashScreen.mockReturnValue({
      ...baseResult,
      status: 'error',
      error: 'DB init failed',
      isVisible: false,
    });

    const { getByText, queryByLabelText } = await render(<SplashScreen />);

    expect(getByText('DB init failed')).toBeTruthy();
    expect(queryByLabelText('Loading')).toBeNull();
  });

  it('shows fallback error message when error is null', async () => {
    mockedUseSplashScreen.mockReturnValue({
      ...baseResult,
      status: 'error',
      error: null,
      isVisible: false,
    });

    const { getByText } = await render(<SplashScreen />);

    expect(getByText('Something went wrong. Please restart the app.')).toBeTruthy();
  });

  it('hides spinner in ready state', async () => {
    mockedUseSplashScreen.mockReturnValue({
      ...baseResult,
      status: 'ready',
      isVisible: false,
    });

    const { queryByLabelText } = await render(<SplashScreen />);

    expect(queryByLabelText('Loading')).toBeNull();
  });

  it('has accessibility label on container', async () => {
    const { getByLabelText } = await render(<SplashScreen />);

    expect(getByLabelText('Loading PayRaider')).toBeTruthy();
  });

  it('has error accessibility label when in error state', async () => {
    mockedUseSplashScreen.mockReturnValue({
      ...baseResult,
      status: 'error',
      error: 'Network error',
      isVisible: false,
    });

    const { getByLabelText } = await render(<SplashScreen />);

    expect(getByLabelText('Initialization error: Network error')).toBeTruthy();
  });
});
