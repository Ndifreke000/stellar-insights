import { renderHook, waitFor } from '@testing-library/react-native';
import { useSplashScreen } from '../useSplashScreen';
import { initializeApp } from '@services/initialization';

jest.mock('@services/initialization', () => ({
  initializeApp: jest.fn(),
}));

const mockedInitializeApp = initializeApp as jest.MockedFunction<typeof initializeApp>;

describe('useSplashScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('starts in loading state', async () => {
    mockedInitializeApp.mockReturnValue(new Promise(() => {})); // never resolves
    const { result } = await renderHook(() => useSplashScreen());

    expect(result.current.status).toBe('loading');
    expect(result.current.isVisible).toBe(true);
    expect(result.current.error).toBeNull();
  });

  it('transitions to ready after successful initialization', async () => {
    mockedInitializeApp.mockResolvedValue();
    const { result } = await renderHook(() => useSplashScreen());

    await waitFor(() => expect(result.current.status).toBe('ready'));

    expect(result.current.isVisible).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('transitions to error state when initialization fails', async () => {
    mockedInitializeApp.mockRejectedValue(new Error('DB init failed'));
    const { result } = await renderHook(() => useSplashScreen());

    await waitFor(() => expect(result.current.status).toBe('error'));

    expect(result.current.isVisible).toBe(false);
    expect(result.current.error).toBe('DB init failed');
  });

  it('uses generic message for non-Error rejections', async () => {
    mockedInitializeApp.mockRejectedValue('string error');
    const { result } = await renderHook(() => useSplashScreen());

    await waitFor(() => expect(result.current.status).toBe('error'));

    expect(result.current.error).toBe('Initialization failed');
  });

  it('exposes a platformName string', async () => {
    mockedInitializeApp.mockReturnValue(new Promise(() => {}));
    const { result } = await renderHook(() => useSplashScreen());

    expect(typeof result.current.platformName).toBe('string');
    expect(result.current.platformName.length).toBeGreaterThan(0);
  });
});
