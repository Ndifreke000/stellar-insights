import { renderHook, act } from '@testing-library/react-native';
import { useHandoffSupport } from '../../features/handoff_support/useHandoffSupport';

jest.mock('react-native', () => ({
  ...jest.requireActual('react-native'),
  Platform: { OS: 'ios' },
}));

describe('useHandoffSupport', () => {
  it('should return supported on iOS', () => {
    const { result } = renderHook(() => useHandoffSupport());
    expect(result.current.supported).toBe(true);
  });

  it('should continue activity successfully', async () => {
    const { result } = renderHook(() => useHandoffSupport());
    const activity = {
      type: 'com.test.activity',
      userInfo: { screen: 'test' },
      needsSave: true,
    };

    await act(async () => {
      await result.current.continueActivity(activity);
    });

    expect(result.current.activity?.type).toBe(activity.type);
    expect(result.current.error).toBeNull();
  });
});
