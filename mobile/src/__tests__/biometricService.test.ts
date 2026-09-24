import {
  authenticate,
  getBiometricType,
  isBiometricAvailable,
} from '@services/biometricService';

interface BiometricsMockHandles {
  isSensorAvailable: jest.Mock;
  simplePrompt: jest.Mock;
}

jest.mock('react-native-biometrics', () => {
  const isSensorAvailable = jest.fn(() =>
    Promise.resolve({ available: false }),
  );
  const simplePrompt = jest.fn(() => Promise.resolve({ success: false }));
  return {
    __esModule: true,
    BiometryTypes: {
      TouchID: 'TouchID',
      FaceID: 'FaceID',
      Biometrics: 'Biometrics',
    },
    default: jest.fn().mockImplementation(() => ({
      isSensorAvailable,
      simplePrompt,
    })),
    __mocks: { isSensorAvailable, simplePrompt },
  };
});

const { isSensorAvailable: mockIsSensorAvailable, simplePrompt: mockSimplePrompt } =
  (jest.requireMock('react-native-biometrics') as {
    __mocks: BiometricsMockHandles;
  }).__mocks;

describe('biometricService', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('isBiometricAvailable', () => {
    it('returns true when a sensor is available', async () => {
      mockIsSensorAvailable.mockResolvedValue({ available: true });
      await expect(isBiometricAvailable()).resolves.toBe(true);
    });

    it('returns false when no sensor is available', async () => {
      mockIsSensorAvailable.mockResolvedValue({ available: false });
      await expect(isBiometricAvailable()).resolves.toBe(false);
    });

    it('returns false when the check throws', async () => {
      mockIsSensorAvailable.mockRejectedValue(new Error('hw error'));
      await expect(isBiometricAvailable()).resolves.toBe(false);
    });
  });

  describe('getBiometricType', () => {
    it('maps FaceID', async () => {
      mockIsSensorAvailable.mockResolvedValue({
        available: true,
        biometryType: 'FaceID',
      });
      await expect(getBiometricType()).resolves.toBe('FaceID');
    });

    it('maps TouchID', async () => {
      mockIsSensorAvailable.mockResolvedValue({
        available: true,
        biometryType: 'TouchID',
      });
      await expect(getBiometricType()).resolves.toBe('TouchID');
    });

    it('maps the generic Android sensor to Fingerprint', async () => {
      mockIsSensorAvailable.mockResolvedValue({
        available: true,
        biometryType: 'Biometrics',
      });
      await expect(getBiometricType()).resolves.toBe('Fingerprint');
    });

    it('returns None when unavailable', async () => {
      mockIsSensorAvailable.mockResolvedValue({ available: false });
      await expect(getBiometricType()).resolves.toBe('None');
    });

    it('returns None when available but type is missing', async () => {
      mockIsSensorAvailable.mockResolvedValue({ available: true });
      await expect(getBiometricType()).resolves.toBe('None');
    });

    it('returns None for an unknown type', async () => {
      mockIsSensorAvailable.mockResolvedValue({
        available: true,
        biometryType: 'Unknown',
      });
      await expect(getBiometricType()).resolves.toBe('None');
    });

    it('returns None when the check throws', async () => {
      mockIsSensorAvailable.mockRejectedValue(new Error('hw error'));
      await expect(getBiometricType()).resolves.toBe('None');
    });
  });

  describe('authenticate', () => {
    it('resolves true on success', async () => {
      mockSimplePrompt.mockResolvedValue({ success: true });
      await expect(authenticate('Sign in')).resolves.toBe(true);
      expect(mockSimplePrompt).toHaveBeenCalledWith({ promptMessage: 'Sign in' });
    });

    it('resolves false when the user cancels', async () => {
      mockSimplePrompt.mockResolvedValue({ success: false });
      await expect(authenticate('Sign in')).resolves.toBe(false);
    });
  });
});
