import ReactNativeBiometrics, {
  BiometryTypes,
} from 'react-native-biometrics';

/** Normalised biometric capability of the device. */
export type BiometricType = 'FaceID' | 'TouchID' | 'Fingerprint' | 'None';

const rnBiometrics = new ReactNativeBiometrics({
  allowDeviceCredentials: false,
});

/**
 * Whether the device has biometric hardware that is enrolled and usable.
 *
 * @returns `true` when biometric authentication can be attempted.
 */
export async function isBiometricAvailable(): Promise<boolean> {
  try {
    const { available } = await rnBiometrics.isSensorAvailable();
    return available;
  } catch {
    return false;
  }
}

/**
 * Resolve the device's biometric type, mapped to a UI-friendly value.
 * Android exposes a single generic `Biometrics` sensor, surfaced as
 * `'Fingerprint'`.
 *
 * @returns The biometric type, or `'None'` when unavailable.
 */
export async function getBiometricType(): Promise<BiometricType> {
  try {
    const { available, biometryType } = await rnBiometrics.isSensorAvailable();
    if (!available || !biometryType) {
      return 'None';
    }
    switch (biometryType) {
      case BiometryTypes.FaceID:
        return 'FaceID';
      case BiometryTypes.TouchID:
        return 'TouchID';
      case BiometryTypes.Biometrics:
        return 'Fingerprint';
      default:
        return 'None';
    }
  } catch {
    return 'None';
  }
}

/**
 * Prompt the user for biometric authentication.
 *
 * @param reason - Message shown in the system biometric prompt.
 * @returns `true` on success; `false` when the user cancels or it fails.
 */
export async function authenticate(reason: string): Promise<boolean> {
  const { success } = await rnBiometrics.simplePrompt({
    promptMessage: reason,
  });
  return success;
}
