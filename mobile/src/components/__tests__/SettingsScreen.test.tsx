import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { SettingsScreen } from '../SettingsScreen';
import { useSettingsScreen } from '@hooks/useSettingsScreen';

jest.mock('@hooks/useSettingsScreen', () => ({
  useSettingsScreen: jest.fn(),
}));

const mockedUseSettingsScreen = useSettingsScreen as jest.MockedFunction<typeof useSettingsScreen>;

describe('SettingsScreen', () => {
  const state = {
    theme: 'light' as const,
    network: 'testnet' as const,
    isOnline: true,
    isSyncing: false,
    platformLabel: 'iOS',
    toggleTheme: jest.fn(),
    toggleNetwork: jest.fn(),
    clearMessage: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
    mockedUseSettingsScreen.mockReturnValue(state);
  });

  it('renders correctly', async () => {
    const { getByText } = await render(<SettingsScreen />);

    expect(getByText('Settings Screen')).toBeTruthy();
  });

  it('shows offline support feedback', async () => {
    mockedUseSettingsScreen.mockReturnValue({ ...state, isOnline: false });

    const { getByText } = await render(<SettingsScreen />);

    expect(getByText('Offline mode active. Changes are saved locally where possible.')).toBeTruthy();
  });

  it('toggles theme from the theme action', async () => {
    const { getByLabelText } = await render(<SettingsScreen />);
    const themeButton = getByLabelText('Toggle theme. Current theme is light');

    await fireEvent.press(themeButton);

    expect(state.toggleTheme).toHaveBeenCalled();
  });
});
