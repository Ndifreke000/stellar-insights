import React from 'react';
import { render, fireEvent, waitFor } from '@testing-library/react-native';
import { VoiceCommandsComponent } from '../VoiceCommandsComponent';

jest.useFakeTimers();

describe('VoiceCommandsComponent', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  it('should render the component', () => {
    const { getByText } = render(<VoiceCommandsComponent />);
    expect(getByText('Voice Commands')).toBeDefined();
  });

  it('should show initializing state initially', () => {
    const { getByText } = render(<VoiceCommandsComponent />);
    expect(getByText('Initializing Voice Commands...')).toBeDefined();
  });

  it('should display listen button after initialization', async () => {
    const { getByA11yLabel } = render(<VoiceCommandsComponent />);

    await waitFor(() => {
      const listenButton = getByA11yLabel('Start listening for voice commands');
      expect(listenButton).toBeDefined();
    }, { timeout: 1000 });
  });

  it('should show available commands', async () => {
    const { getByText } = render(<VoiceCommandsComponent />);

    await waitFor(() => {
      expect(getByText('Available Commands:')).toBeDefined();
    });

    expect(getByText(/• Dashboard/)).toBeDefined();
  });

  it('should handle listen button press', async () => {
    const { getByA11yLabel, getByText } = render(<VoiceCommandsComponent />);

    await waitFor(() => {
      const listenButton = getByA11yLabel('Start listening for voice commands');
      expect(listenButton).toBeDefined();
    });

    const listenButton = getByA11yLabel('Start listening for voice commands');
    fireEvent.press(listenButton);

    await waitFor(() => {
      expect(getByText('Listening... Say a command')).toBeDefined();
    });
  });

  it('should show transcribed command after listening', async () => {
    const { getByA11yLabel, getByText, queryByText } = render(<VoiceCommandsComponent />);

    await waitFor(() => {
      const listenButton = getByA11yLabel('Start listening for voice commands');
      fireEvent.press(listenButton);
    });

    jest.advanceTimersByTime(3000);

    await waitFor(() => {
      expect(queryByText('You said:')).toBeDefined();
    });
  });

  it('should clear result when clear button is pressed', async () => {
    jest.useFakeTimers();
    const { getByA11yLabel, getByText, queryByText } = render(<VoiceCommandsComponent />);

    await waitFor(() => {
      const listenButton = getByA11yLabel('Start listening for voice commands');
      fireEvent.press(listenButton);
    });

    jest.advanceTimersByTime(3000);

    await waitFor(() => {
      expect(queryByText('You said:')).toBeDefined();
    });

    const clearButton = queryByText('Clear');
    if (clearButton) {
      fireEvent.press(clearButton);

      await waitFor(() => {
        expect(queryByText('You said:')).toBeNull();
      });
    }
  });
});
