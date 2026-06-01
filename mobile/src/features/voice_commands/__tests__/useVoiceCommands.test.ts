import { renderHook, act, waitFor } from '@testing-library/react-native';
import { useVoiceCommands } from '../useVoiceCommands';

jest.useFakeTimers();

describe('useVoiceCommands', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  afterEach(() => {
    jest.clearAllTimers();
  });

  it('should initialize with correct default state', () => {
    const { result } = renderHook(() => useVoiceCommands());

    expect(result.current.isListening).toBe(false);
    expect(result.current.transcribedText).toBeNull();
    expect(result.current.error).toBeNull();
    expect(result.current.commandResult).toBeNull();
  });

  it('should return supported commands', () => {
    const { result } = renderHook(() => useVoiceCommands());

    const commands = result.current.getSupportedCommands();

    expect(Array.isArray(commands)).toBe(true);
    expect(commands.length).toBeGreaterThan(0);
    expect(commands).toContain('Dashboard');
  });

  it('should start and stop listening', async () => {
    const { result } = renderHook(() => useVoiceCommands());

    await act(async () => {
      await result.current.startListening();
    });

    expect(result.current.isListening).toBe(true);

    await act(async () => {
      await result.current.stopListening();
    });

    expect(result.current.isListening).toBe(false);
  });

  it('should transcribe voice command', async () => {
    const { result } = renderHook(() => useVoiceCommands());

    await act(async () => {
      await result.current.startListening();
    });

    jest.advanceTimersByTime(3000);

    await waitFor(() => {
      expect(result.current.transcribedText).not.toBeNull();
    });

    expect(result.current.isListening).toBe(false);
  });

  it('should process voice command', async () => {
    const { result } = renderHook(() => useVoiceCommands());

    await act(async () => {
      await result.current.startListening();
    });

    jest.advanceTimersByTime(3000);

    await waitFor(() => {
      expect(result.current.commandResult).not.toBeNull();
    });

    expect(result.current.isListening).toBe(false);
  });

  it('should reset result', async () => {
    const { result } = renderHook(() => useVoiceCommands());

    await act(async () => {
      await result.current.startListening();
    });

    jest.advanceTimersByTime(3000);

    await waitFor(() => {
      expect(result.current.transcribedText).not.toBeNull();
    });

    act(() => {
      result.current.resetResult();
    });

    expect(result.current.transcribedText).toBeNull();
    expect(result.current.commandResult).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('should handle permission checks', async () => {
    const { result } = renderHook(() => useVoiceCommands());

    const hasPermission = await act(async () => {
      return await result.current.hasPermission();
    });

    expect(typeof hasPermission).toBe('boolean');
  });
});
