import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { OfflineQueue } from '../OfflineQueue';
import { useOfflineQueue } from '@hooks/useOfflineQueue';
import { useAppStore } from '@store/appStore';

jest.mock('@hooks/useOfflineQueue', () => ({
  useOfflineQueue: jest.fn(),
}));

jest.mock('@store/appStore', () => ({
  useAppStore: jest.fn(),
}));

const mockedUseOfflineQueue = useOfflineQueue as jest.MockedFunction<typeof useOfflineQueue>;
const mockedUseAppStore = useAppStore as unknown as jest.Mock;

describe('OfflineQueue', () => {
  const queueState = {
    items: [],
    isProcessing: false,
    enqueue: jest.fn(),
    remove: jest.fn(),
    clear: jest.fn(),
    retryFailed: jest.fn(),
    processQueue: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
    mockedUseOfflineQueue.mockReturnValue(queueState);
    mockedUseAppStore.mockImplementation(selector => selector({ isOnline: true }));
  });

  it('renders correctly', async () => {
    const { getByText } = await render(<OfflineQueue />);

    expect(getByText('Offline Queue')).toBeTruthy();
  });

  it('shows offline feedback when the device is offline', async () => {
    mockedUseAppStore.mockImplementation(selector => selector({ isOnline: false }));

    const { getByText } = await render(<OfflineQueue />);

    expect(getByText('Offline mode active')).toBeTruthy();
  });

  it('renders queued requests and removes an item', async () => {
    const remove = jest.fn();
    mockedUseOfflineQueue.mockReturnValue({
      ...queueState,
      remove,
      items: [
        {
          id: 'queued-request',
          method: 'POST',
          url: '/payments',
          payload: { amount: 10 },
          retryCount: 1,
          status: 'failed',
          createdAt: '2026-01-01T00:00:00.000Z',
          updatedAt: '2026-01-01T00:00:00.000Z',
          lastError: 'Network unavailable',
        },
      ],
    });

    const { getByLabelText } = await render(<OfflineQueue />);
    const removeButton = getByLabelText('Remove queued request queued-request');

    await fireEvent.press(removeButton);

    expect(remove).toHaveBeenCalledWith('queued-request');
  });
});
