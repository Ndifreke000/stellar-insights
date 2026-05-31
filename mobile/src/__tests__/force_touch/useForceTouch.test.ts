import { renderHook, act } from '@testing-library/react-native';
import { useForceTouch } from '../../features/force_touch/useForceTouch';
import { Platform } from 'react-native';

jest.mock('react-native', () => ({
  ...jest.requireActual('react-native'),
  Platform: { OS: 'ios' },
  GestureResponderEvent: {},
}));

describe('useForceTouch', () => {
  it('should return supported on iOS', () => {
    const { result } = renderHook(() => useForceTouch());
    expect(result.current.supported).toBe(true);
  });

  it('should handle press events', async () => {
    const { result } = renderHook(() => useForceTouch());
    const mockEvent = {
      nativeEvent: { force: 0.5, locationX: 100, locationY: 200 },
    } as any;

    await act(async () => {
      result.current.handlePress(mockEvent);
    });

    expect(result.current.touchData?.force).toBeCloseTo(0.5);
    expect(result.current.error).toBeNull();
  });

  it('should clamp force to 0-1 range', async () => {
    const { result } = renderHook(() => useForceTouch());
    const mockEvent = {
      nativeEvent: { force: 1.5, locationX: 0, locationY: 0 },
    } as any;

    await act(async () => {
      result.current.handlePress(mockEvent);
    });

    expect(result.current.touchData?.force).toBeLessThanOrEqual(1);
  });
});
