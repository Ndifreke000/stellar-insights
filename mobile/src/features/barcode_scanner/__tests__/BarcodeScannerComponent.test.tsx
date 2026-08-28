import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react-native';
import { BarcodeScannerComponent } from '../BarcodeScannerComponent';

jest.useFakeTimers();

describe('BarcodeScannerComponent', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  it('should render the component', () => {
    const { getByText } = render(<BarcodeScannerComponent />);
    expect(getByText('Barcode Scanner')).toBeDefined();
  });

  it('should show initializing state initially', () => {
    const { getByText } = render(<BarcodeScannerComponent />);
    expect(getByText('Initializing Barcode Scanner...')).toBeDefined();
  });

  it('should display start scan button after initialization', async () => {
    const { getByText } = render(<BarcodeScannerComponent />);

    await waitFor(() => {
      const startButton = getByText('Start Scan');
      expect(startButton).toBeDefined();
    }, { timeout: 1000 });
  });

  it('should handle scan button press', async () => {
    const { getByText, getByLabelText } = render(<BarcodeScannerComponent />);

    await waitFor(() => {
      const startButton = getByLabelText('Start barcode scan');
      expect(startButton).toBeDefined();
    });

    const startButton = getByLabelText('Start barcode scan');
    fireEvent.press(startButton);

    await waitFor(() => {
      expect(getByText('Scanning... Point at a barcode')).toBeDefined();
    });
  });

  it('should show result after scan completes', async () => {
    const { getByLabelText, getByText } = render(<BarcodeScannerComponent />);

    await waitFor(() => {
      const startButton = getByLabelText('Start barcode scan');
      expect(startButton).toBeDefined();
      fireEvent.press(startButton);
    });

    jest.advanceTimersByTime(3000);

    await waitFor(() => {
      expect(getByText('Scanned Barcode:')).toBeDefined();
    });
  });

  it('should clear result when clear button is pressed', async () => {
    jest.useFakeTimers();
    const { getByLabelText, getByText, queryByText } = render(<BarcodeScannerComponent />);

    await waitFor(() => {
      const startButton = getByLabelText('Start barcode scan');
      fireEvent.press(startButton);
    });

    jest.advanceTimersByTime(3000);

    await waitFor(() => {
      expect(getByText('Scanned Barcode:')).toBeDefined();
    });

    const clearButton = getByLabelText('Clear result');
    fireEvent.press(clearButton);

    await waitFor(() => {
      expect(queryByText('Scanned Barcode:')).toBeNull();
    });
  });
});
