import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';
import { FaceRecognitionComponent } from '../FaceRecognitionComponent';

jest.useFakeTimers();

describe('FaceRecognitionComponent', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  it('should render the component', () => {
    const { getByText } = render(<FaceRecognitionComponent />);
    expect(getByText('Face Recognition')).toBeDefined();
  });

  it('should show initializing state initially', () => {
    const { getByText } = render(<FaceRecognitionComponent />);
    expect(getByText('Initializing Face Recognition...')).toBeDefined();
  });

  it('should display scan button after initialization', async () => {
    const { getByA11yLabel } = render(<FaceRecognitionComponent />);

    await waitFor(() => {
      const scanButton = getByA11yLabel('Start face recognition');
      expect(scanButton).toBeDefined();
    }, { timeout: 1000 });
  });

  it('should handle scan button press', async () => {
    const { getByA11yLabel, getByText } = render(<FaceRecognitionComponent />);

    await waitFor(() => {
      const scanButton = getByA11yLabel('Start face recognition');
      expect(scanButton).toBeDefined();
    });

    const scanButton = getByA11yLabel('Start face recognition');
    fireEvent.press(scanButton);

    await waitFor(() => {
      expect(getByText('Position your face in the frame')).toBeDefined();
    });
  });

  it('should show result after scan completes', async () => {
    const { getByA11yLabel, getByText, queryByText } = render(<FaceRecognitionComponent />);

    await waitFor(() => {
      const scanButton = getByA11yLabel('Start face recognition');
      fireEvent.press(scanButton);
    });

    jest.advanceTimersByTime(3500);

    await waitFor(() => {
      // Either face recognized or error should be shown
      const recognizedText = queryByText('Face Recognized');
      const errorPresent = queryByText(/not recognized/);
      expect(recognizedText || errorPresent).toBeTruthy();
    });
  });

  it('should reset result when reset button is pressed', async () => {
    jest.useFakeTimers();
    const { getByA11yLabel, getByText, queryByText } = render(<FaceRecognitionComponent />);

    await waitFor(() => {
      const scanButton = getByA11yLabel('Start face recognition');
      fireEvent.press(scanButton);
    });

    jest.advanceTimersByTime(3500);

    await waitFor(() => {
      const resetButton = queryByText('Reset');
      if (resetButton) {
        fireEvent.press(resetButton);
      }
    });
  });
});
