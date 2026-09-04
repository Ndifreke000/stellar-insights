import { act, renderHook, waitFor } from '@testing-library/react-native';

import { useSecureTokenStorage } from '@hooks/useSecureTokenStorage';
import {
  getToken,
  removeToken,
  saveToken,
} from '@services/auth';

jest.mock('@services/auth', () => ({
  getToken: jest.fn(),
  saveToken: jest.fn(),
  removeToken: jest.fn(),
}));

const mockGetToken = getToken as jest.Mock;
const mockSaveToken = saveToken as jest.Mock;
const mockRemoveToken = removeToken as jest.Mock;

describe('useSecureTokenStorage', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockGetToken.mockResolvedValue(null);
    mockSaveToken.mockResolvedValue(undefined);
    mockRemoveToken.mockResolvedValue(undefined);
  });

  it('loads the stored token on mount', async () => {
    mockGetToken.mockResolvedValue('stored-token');

    const { result } = await renderHook(() => useSecureTokenStorage());
    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.token).toBe('stored-token');
    expect(result.current.error).toBeNull();
  });

  it('exposes a null token when none is stored', async () => {
    const { result } = await renderHook(() => useSecureTokenStorage());
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.token).toBeNull();
  });

  it('saves a token and reflects it in state', async () => {
    const { result } = await renderHook(() => useSecureTokenStorage());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.saveToken('new-token', 123);
    });

    expect(mockSaveToken).toHaveBeenCalledWith('new-token', 123);
    expect(result.current.token).toBe('new-token');
    expect(result.current.error).toBeNull();
  });

  it('removes a token and clears state', async () => {
    mockGetToken.mockResolvedValue('stored-token');
    const { result } = await renderHook(() => useSecureTokenStorage());
    await waitFor(() => expect(result.current.token).toBe('stored-token'));

    await act(async () => {
      await result.current.removeToken();
    });

    expect(mockRemoveToken).toHaveBeenCalledTimes(1);
    expect(result.current.token).toBeNull();
  });

  it('surfaces a read error without throwing', async () => {
    mockGetToken.mockRejectedValue(new Error('read failed'));

    const { result } = await renderHook(() => useSecureTokenStorage());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.token).toBeNull();
    expect(result.current.error).toBe('Failed to read secure storage');
  });

  it('surfaces a save error without throwing', async () => {
    mockSaveToken.mockRejectedValue(new Error('write failed'));
    const { result } = await renderHook(() => useSecureTokenStorage());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.saveToken('new-token');
    });

    expect(result.current.error).toBe('Failed to save secure storage');
    expect(result.current.token).toBeNull();
  });

  it('surfaces a remove error without throwing', async () => {
    mockRemoveToken.mockRejectedValue(new Error('remove failed'));
    const { result } = await renderHook(() => useSecureTokenStorage());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.removeToken();
    });

    expect(result.current.error).toBe('Failed to clear secure storage');
  });
});
