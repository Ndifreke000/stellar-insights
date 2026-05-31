import { renderHook, act } from '@testing-library/react-native';
import { useGeofencing } from '../../features/geofencing/useGeofencing';

jest.mock('react-native', () => ({
  ...jest.requireActual('react-native'),
  Platform: { OS: 'ios' },
  PermissionsAndroid: { request: jest.fn(() => Promise.resolve('granted')) },
}));

describe('useGeofencing', () => {
  it('should return supported on iOS and Android', () => {
    const { result } = renderHook(() => useGeofencing());
    expect(result.current.supported).toBe(true);
  });

  it('should add geofence successfully', async () => {
    const { result } = renderHook(() => useGeofencing());
    const fence = {
      id: 'test-fence',
      latitude: 37.7749,
      longitude: -122.4194,
      radius: 100,
      name: 'San Francisco',
    };

    await act(async () => {
      await result.current.addFence(fence);
    });

    expect(result.current.fences).toContainEqual(fence);
    expect(result.current.error).toBeNull();
  });

  it('should remove geofence successfully', async () => {
    const { result } = renderHook(() => useGeofencing());
    const fence = {
      id: 'test-fence',
      latitude: 37.7749,
      longitude: -122.4194,
      radius: 100,
      name: 'San Francisco',
    };

    await act(async () => {
      await result.current.addFence(fence);
      await result.current.removeFence('test-fence');
    });

    expect(result.current.fences).toHaveLength(0);
  });
});
