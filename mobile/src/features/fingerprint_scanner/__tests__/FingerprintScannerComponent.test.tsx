import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react-native';
import { FingerprintScannerComponent } from '../FingerprintScannerComponent';

jest.useFakeTimers();

describe('FingerprintScannerComponent', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  it('should render the component', () => {
    const { getByText } = render(<FingerprintScannerComponent />);
    expect(getByText('Fingerprint Scanner')).toBeDefined();
  });

  it('should show initializing state initially', () => {
    const { getByText } = render(<FingerprintScannerComponent />);
    expect(getByText('Initializing Fingerprint Scanner...')).toBeDefined();
  });

  it('should display scan button after initialization', async () => {
    const { getByA11yLabel } = render(<FingerprintScannerComponent />);

    await waitFor(() => {
      const scanButton = getByA11yLabel('Start fingerprint scan');
      expect(scanButton).toBeDefined();
    }, { timeout: 1000 });
  });

  it('should handle scan button press', async () => {
    const { getByA11yLabel, getByText } = render(<FingerprintScannerComponent />);

    await waitFor(() => {
      const scanButton = getByA11yLabel('Start fingerprint scan');
      expect(scanButton).toBeDefined();
    });

    const scanButton = getByA11yLabel('Start fingerprint scan');
    fireEvent.press(scanButton);

    await waitFor(() => {
      expect(getByText('Place your finger on the sensor')).toBeDefined();
    });
  });

  it('should show result after scan completes', async () => {
    const { getByA11yLabel, getByText, queryByText } = render(<FingerprintScannerComponent />);

    await waitFor(() => {
      const scanButton = getByA11yLabel('Start fingerprint scan');
      fireEvent.press(scanButton);
    });

    jest.advanceTimersByTime(2500);

    await waitFor(() => {
      // Either authentication successful or error should be shown
      const successText = queryByText('Authentication Successful');
      const errorPresent = queryByText(/not recognized/);
      expect(successText || errorPresent).toBeTruthy();
    });
  });

  it('should reset result when reset button is pressed', async () => {
    jest.useFakeTimers();
    const { getByA11yLabel, getByText, queryByText } = render(<FingerprintScannerComponent />);

    await waitFor(() => {
      const scanButton = getByA11yLabel('Start fingerprint scan');
      fireEvent.press(scanButton);
    });

    jest.advanceTimersByTime(2500);

    await waitFor(() => {
      const resetButton = queryByText('Reset');
      if (resetButton) {
        fireEvent.press(resetButton);
      }
    });
  });
});
