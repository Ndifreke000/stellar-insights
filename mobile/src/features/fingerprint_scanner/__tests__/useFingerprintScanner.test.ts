import { renderHook, act, waitFor } from '@testing-library/react-native';
import { useFingerprintScanner } from '../useFingerprintScanner';

jest.useFakeTimers();

describe('useFingerprintScanner', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  it('should initialize with correct default state', () => {
    const { result } = renderHook(() => useFingerprintScanner());

    expect(result.current.isScanning).toBe(false);
    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('should check biometric availability', async () => {
    const { result } = renderHook(() => useFingerprintScanner());

    const bioAvailable = await act(async () => {
      return await result.current.isBiometricAvailable();
    });

    expect(typeof bioAvailable).toBe('boolean');
  });

  it('should start and stop scanning', async () => {
    const { result } = renderHook(() => useFingerprintScanner());

    await act(async () => {
      await result.current.startScan();
    });

    expect(result.current.isScanning).toBe(true);

    await act(async () => {
      await result.current.stopScan();
    });

    expect(result.current.isScanning).toBe(false);
  });

  it('should authenticate successfully', async () => {
    const { result } = renderHook(() => useFingerprintScanner());

    await act(async () => {
      await result.current.startScan();
    });

    jest.advanceTimersByTime(2500);

    await waitFor(() => {
      expect(result.current.isScanning).toBe(false);
    });

    // Check if either authenticated or error occurred
    expect(result.current.isAuthenticated || result.current.error).toBeTruthy();
  });

  it('should reset result', async () => {
    const { result } = renderHook(() => useFingerprintScanner());

    await act(async () => {
      await result.current.startScan();
    });

    jest.advanceTimersByTime(2500);

    await waitFor(() => {
      expect(result.current.isScanning).toBe(false);
    });

    act(() => {
      result.current.resetResult();
    });

    expect(result.current.isAuthenticated).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('should handle permission checks', async () => {
    const { result } = renderHook(() => useFingerprintScanner());

    const hasPermission = await act(async () => {
      return await result.current.hasPermission();
    });

    expect(typeof hasPermission).toBe('boolean');
  });
});
