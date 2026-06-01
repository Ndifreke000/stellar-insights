import { renderHook, act, waitFor } from '@testing-library/react-native';
import { useBarcodeScanner } from '../useBarcodeScanner';

jest.useFakeTimers();

describe('useBarcodeScanner', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  it('should initialize with correct default state', () => {
    const { result } = renderHook(() => useBarcodeScanner());

    expect(result.current.isScanning).toBe(false);
    expect(result.current.result).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('should start and stop scanning', async () => {
    const { result } = renderHook(() => useBarcodeScanner());

    await act(async () => {
      await result.current.startScan();
    });

    expect(result.current.isScanning).toBe(true);

    await act(async () => {
      await result.current.stopScan();
    });

    expect(result.current.isScanning).toBe(false);
  });

  it('should return barcode result after scan', async () => {
    const { result } = renderHook(() => useBarcodeScanner());

    await act(async () => {
      await result.current.startScan();
    });

    jest.advanceTimersByTime(3000);

    await waitFor(() => {
      expect(result.current.result).not.toBeNull();
    });

    expect(result.current.result?.data).toBeDefined();
    expect(result.current.result?.type).toBeDefined();
    expect(result.current.isScanning).toBe(false);
  });

  it('should reset result', async () => {
    const { result } = renderHook(() => useBarcodeScanner());

    await act(async () => {
      await result.current.startScan();
    });

    jest.advanceTimersByTime(3000);

    await waitFor(() => {
      expect(result.current.result).not.toBeNull();
    });

    act(() => {
      result.current.resetResult();
    });

    expect(result.current.result).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('should handle permission checks', async () => {
    const { result } = renderHook(() => useBarcodeScanner());

    const hasPermission = await act(async () => {
      return await result.current.hasPermission();
    });

    expect(typeof hasPermission).toBe('boolean');
  });

  it('should request permission', async () => {
    const { result } = renderHook(() => useBarcodeScanner());

    const permissionGranted = await act(async () => {
      return await result.current.requestPermission();
    });

    expect(typeof permissionGranted).toBe('boolean');
  });
});
