import { renderHook, act } from '@testing-library/react-native';
import { useBackgroundSync } from '../../features/background_sync/useBackgroundSync';

jest.mock('react-native', () => ({
  ...jest.requireActual('react-native'),
  Platform: { OS: 'ios' },
  AppState: { addEventListener: jest.fn(() => ({ remove: jest.fn() })) },
}));

describe('useBackgroundSync', () => {
  it('should return supported on iOS and Android', () => {
    const { result } = renderHook(() => useBackgroundSync());
    expect(result.current.supported).toBe(true);
  });

  it('should queue task successfully', async () => {
    const { result } = renderHook(() => useBackgroundSync());

    await act(async () => {
      await result.current.queueTask({
        type: 'sync_test',
        data: { test: true },
      });
    });

    expect(result.current.tasks).toHaveLength(1);
    expect(result.current.tasks[0].status).toBe('pending');
  });

  it('should sync tasks successfully', async () => {
    const { result } = renderHook(() => useBackgroundSync());

    await act(async () => {
      await result.current.queueTask({
        type: 'sync_test',
        data: { test: true },
      });
      await result.current.syncNow();
    });

    expect(result.current.tasks[0].status).toBe('completed');
  });
});
