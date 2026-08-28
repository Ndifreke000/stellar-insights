import React from 'react';
import { render } from '@testing-library/react-native';
import { GestureControlsComponent } from '@components/GestureControlsComponent';

jest.mock('@react-native-community/netinfo', () => ({
  useNetInfo: jest.fn(() => ({ isConnected: true, isInternetReachable: true })),
}));

describe('GestureControlsComponent', () => {
  it('renders the gesture controls screen and status labels', () => {
    const { getByText, getByLabelText } = render(<GestureControlsComponent />);

    expect(getByText('Gesture Controls')).toBeTruthy();
    expect(getByText('Ready for input')).toBeTruthy();
    expect(getByText('Gesture count: 0')).toBeTruthy();
    expect(getByLabelText('Gesture interaction area')).toBeTruthy();
  });

  it('shows offline mode when network is unavailable', () => {
    const netInfo = require('@react-native-community/netinfo');
    netInfo.useNetInfo.mockReturnValueOnce({ isConnected: false, isInternetReachable: false });

    const { getByText } = render(<GestureControlsComponent />);

    expect(getByText('Offline mode active')).toBeTruthy();
  });
});
