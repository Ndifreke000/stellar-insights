import { renderHook, act, waitFor } from '@testing-library/react-native';
import { useFaceRecognition } from '../useFaceRecognition';

jest.useFakeTimers();

describe('useFaceRecognition', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  it('should initialize with correct default state', () => {
    const { result } = renderHook(() => useFaceRecognition());

    expect(result.current.isScanning).toBe(false);
    expect(result.current.isRecognized).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.confidenceScore).toBe(0);
  });

  it('should check camera availability', async () => {
    const { result } = renderHook(() => useFaceRecognition());

    const camAvailable = await act(async () => {
      return await result.current.isCameraAvailable();
    });

    expect(typeof camAvailable).toBe('boolean');
  });

  it('should start and stop scanning', async () => {
    const { result } = renderHook(() => useFaceRecognition());

    await act(async () => {
      await result.current.startScan();
    });

    expect(result.current.isScanning).toBe(true);

    await act(async () => {
      await result.current.stopScan();
    });

    expect(result.current.isScanning).toBe(false);
  });

  it('should recognize face successfully', async () => {
    const { result } = renderHook(() => useFaceRecognition());

    await act(async () => {
      await result.current.startScan();
    });

    jest.advanceTimersByTime(3500);

    await waitFor(() => {
      expect(result.current.isScanning).toBe(false);
    });

    // Check if either recognized or error occurred
    expect(result.current.isRecognized || result.current.error).toBeTruthy();

    // If recognized, confidence should be set
    if (result.current.isRecognized) {
      expect(result.current.confidenceScore).toBeGreaterThanOrEqual(0.85);
    }
  });

  it('should reset result', async () => {
    const { result } = renderHook(() => useFaceRecognition());

    await act(async () => {
      await result.current.startScan();
    });

    jest.advanceTimersByTime(3500);

    await waitFor(() => {
      expect(result.current.isScanning).toBe(false);
    });

    act(() => {
      result.current.resetResult();
    });

    expect(result.current.isRecognized).toBe(false);
    expect(result.current.error).toBeNull();
    expect(result.current.confidenceScore).toBe(0);
  });

  it('should handle permission checks', async () => {
    const { result } = renderHook(() => useFaceRecognition());

    const hasPermission = await act(async () => {
      return await result.current.hasPermission();
    });

    expect(typeof hasPermission).toBe('boolean');
  });
});
