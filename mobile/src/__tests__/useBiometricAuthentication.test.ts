import { act, renderHook, waitFor } from '@testing-library/react-native';

import { useBiometricAuthentication } from '@hooks/useBiometricAuthentication';
import {
  authenticate,
  getBiometricType,
  isBiometricAvailable,
} from '@services/biometricService';

jest.mock('@services/biometricService', () => ({
  isBiometricAvailable: jest.fn(),
  getBiometricType: jest.fn(),
  authenticate: jest.fn(),
}));

const mockIsAvailable = isBiometricAvailable as jest.Mock;
const mockGetType = getBiometricType as jest.Mock;
const mockAuthenticate = authenticate as jest.Mock;

describe('useBiometricAuthentication', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockIsAvailable.mockResolvedValue(false);
    mockGetType.mockResolvedValue('None');
    mockAuthenticate.mockResolvedValue(false);
  });

  it('reports availability and type on mount', async () => {
    mockIsAvailable.mockResolvedValue(true);
    mockGetType.mockResolvedValue('FaceID');

    const { result } = await renderHook(() => useBiometricAuthentication());

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.isAvailable).toBe(true);
    expect(result.current.biometricType).toBe('FaceID');
    expect(result.current.error).toBeNull();
  });

  it('reports unavailable biometrics', async () => {
    const { result } = await renderHook(() => useBiometricAuthentication());
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.isAvailable).toBe(false);
    expect(result.current.biometricType).toBe('None');
  });

  it('resolves true when authentication succeeds', async () => {
    mockAuthenticate.mockResolvedValue(true);
    const { result } = await renderHook(() => useBiometricAuthentication());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    let outcome = false;
    await act(async () => {
      outcome = await result.current.authenticate('Sign in');
    });
    expect(outcome).toBe(true);
    expect(result.current.error).toBeNull();
  });

  it('resolves false when the user cancels', async () => {
    mockAuthenticate.mockResolvedValue(false);
    const { result } = await renderHook(() => useBiometricAuthentication());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    let outcome = true;
    await act(async () => {
      outcome = await result.current.authenticate('Sign in');
    });
    expect(outcome).toBe(false);
  });

  it('surfaces a hardware error via the error field and resolves false', async () => {
    mockAuthenticate.mockRejectedValue(new Error('hardware error'));
    const { result } = await renderHook(() => useBiometricAuthentication());
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    let outcome = true;
    await act(async () => {
      outcome = await result.current.authenticate('Sign in');
    });
    expect(outcome).toBe(false);
    expect(result.current.error).toBe(
      'Biometric authentication is unavailable right now',
    );
  });

  it('surfaces an availability-probe error', async () => {
    mockIsAvailable.mockRejectedValue(new Error('probe failed'));
    const { result } = await renderHook(() => useBiometricAuthentication());
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.error).toBe('Unable to check biometric availability');
  });
});
