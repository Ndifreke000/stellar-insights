import { useCallback, useEffect, useMemo, useState } from 'react';
import axios from 'axios';

import { apiClient } from '@services/api';
import { getToken, saveToken } from '@services/tokenStorage';
import { useBiometricAuthentication } from '@hooks/useBiometricAuthentication';
import type { BiometricType } from '@services/biometricService';
import { useAuthStore } from '@store/authStore';
import { AuthTokens, User } from '@app-types/index';

/** Credentials submitted from the login form. */
export interface LoginCredentials {
  identifier: string;
  password: string;
}

/** Field-level validation messages, keyed by form field. */
export interface LoginFieldErrors {
  identifier?: string;
  password?: string;
}

/** Shape of a successful `/auth/login` response. */
export interface LoginResponse {
  tokens: AuthTokens;
  user: User;
}

/** Options accepted by {@link useLoginScreen}. */
export interface UseLoginScreenOptions {
  /** Invoked after a successful authentication. */
  onLoginSuccess?: () => void;
}

/** Public API returned by {@link useLoginScreen}. */
export interface UseLoginScreenReturn {
  identifier: string;
  password: string;
  showPassword: boolean;
  fieldErrors: LoginFieldErrors;
  formError: string | null;
  loading: boolean;
  isValid: boolean;
  setIdentifier: (value: string) => void;
  setPassword: (value: string) => void;
  toggleShowPassword: () => void;
  handleSubmit: () => Promise<void>;
  /** True only for returning users: biometrics available AND a token is stored. */
  canUseBiometrics: boolean;
  /** Platform label for the biometric prompt, e.g. "Face ID". */
  biometricLabel: string;
  /** Whether the biometric availability probe is still running. */
  biometricLoading: boolean;
  handleBiometricLogin: () => Promise<void>;
}

/**
 * Map a biometric type to its user-facing platform label.
 *
 * @param type - Detected biometric type.
 * @returns A display label such as "Face ID", "Touch ID", or "Fingerprint".
 */
export function getBiometricLabel(type: BiometricType): string {
  switch (type) {
    case 'FaceID':
      return 'Face ID';
    case 'TouchID':
      return 'Touch ID';
    case 'Fingerprint':
      return 'Fingerprint';
    default:
      return 'Biometrics';
  }
}

const EMAIL_REGEX = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const MIN_USERNAME_LENGTH = 3;
const MIN_PASSWORD_LENGTH = 6;

/**
 * Validate the identifier (email or username) field.
 *
 * @param value - Raw identifier input.
 * @returns A validation message, or `undefined` when valid.
 */
export function validateIdentifier(value: string): string | undefined {
  const trimmed = value.trim();
  if (!trimmed) {
    return 'Email or username is required';
  }
  if (trimmed.includes('@') && !EMAIL_REGEX.test(trimmed)) {
    return 'Enter a valid email address';
  }
  if (!trimmed.includes('@') && trimmed.length < MIN_USERNAME_LENGTH) {
    return `Username must be at least ${MIN_USERNAME_LENGTH} characters`;
  }
  return undefined;
}

/**
 * Validate the password field.
 *
 * @param value - Raw password input.
 * @returns A validation message, or `undefined` when valid.
 */
export function validatePassword(value: string): string | undefined {
  if (!value) {
    return 'Password is required';
  }
  if (value.length < MIN_PASSWORD_LENGTH) {
    return `Password must be at least ${MIN_PASSWORD_LENGTH} characters`;
  }
  return undefined;
}

/**
 * Translate a login failure into a user-facing, non-alarming message.
 *
 * @param error - The error thrown by the auth request.
 * @returns A human-readable error string.
 */
export function getLoginErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    if (!error.response) {
      return 'Unable to connect. Check your internet connection and try again.';
    }
    if (error.response.status === 401 || error.response.status === 403) {
      return 'Incorrect email/username or password.';
    }
  }
  return 'Something went wrong. Please try again.';
}

/**
 * Hook that owns all login-form state: input values, password visibility,
 * field-level and global validation/error state, loading status, and the
 * submit handler that authenticates against the API and updates the auth store.
 *
 * @param options - Optional success callback.
 * @returns State and handlers for rendering a login screen.
 */
export function useLoginScreen(
  options: UseLoginScreenOptions = {},
): UseLoginScreenReturn {
  const { onLoginSuccess } = options;

  const [identifier, setIdentifierState] = useState('');
  const [password, setPasswordState] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<LoginFieldErrors>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const setIdentifier = useCallback((value: string) => {
    setIdentifierState(value);
    setFieldErrors(prev => ({ ...prev, identifier: undefined }));
    setFormError(null);
  }, []);

  const setPassword = useCallback((value: string) => {
    setPasswordState(value);
    setFieldErrors(prev => ({ ...prev, password: undefined }));
    setFormError(null);
  }, []);

  const toggleShowPassword = useCallback(() => {
    setShowPassword(prev => !prev);
  }, []);

  const [hasStoredToken, setHasStoredToken] = useState(false);
  const biometrics = useBiometricAuthentication();

  useEffect(() => {
    let active = true;
    void (async () => {
      try {
        const token = await getToken();
        if (active) {
          setHasStoredToken(!!token);
        }
      } catch {
        if (active) {
          setHasStoredToken(false);
        }
      }
    })();
    return () => {
      active = false;
    };
  }, []);

  const biometricLabel = getBiometricLabel(biometrics.biometricType);
  const canUseBiometrics = biometrics.isAvailable && hasStoredToken;

  const isValid = useMemo(
    () => !validateIdentifier(identifier) && !validatePassword(password),
    [identifier, password],
  );

  const handleBiometricLogin = useCallback(async () => {
    setFormError(null);

    const succeeded = await biometrics.authenticate(
      `Sign in with ${biometricLabel}`,
    );
    if (!succeeded) {
      setFormError(
        `${biometricLabel} sign-in didn’t complete. Enter your credentials to continue.`,
      );
      return;
    }

    const token = await getToken();
    if (!token) {
      setFormError(
        'Your saved sign-in is no longer available. Enter your credentials to continue.',
      );
      return;
    }

    // Returning-user flow: reuse the stored token; do not re-issue credentials.
    useAuthStore.setState({ isAuthenticated: true });
    onLoginSuccess?.();
  }, [biometrics, biometricLabel, onLoginSuccess]);

  const handleSubmit = useCallback(async () => {
    const errors: LoginFieldErrors = {
      identifier: validateIdentifier(identifier),
      password: validatePassword(password),
    };

    if (errors.identifier || errors.password) {
      setFieldErrors(errors);
      return;
    }

    setFieldErrors({});
    setFormError(null);
    setLoading(true);

    try {
      const response = await apiClient.post<LoginResponse>('/auth/login', {
        identifier: identifier.trim(),
        password,
      });

      useAuthStore.getState().setTokens(response.tokens);
      useAuthStore.getState().setUser(response.user);

      // Persist the token securely; storage failure must not block sign-in.
      try {
        await saveToken(
          response.tokens.accessToken,
          response.tokens.expiresAt,
        );
      } catch {
        // Non-fatal: the user is authenticated for this session regardless.
      }

      onLoginSuccess?.();
    } catch (error) {
      setFormError(getLoginErrorMessage(error));
    } finally {
      setLoading(false);
    }
  }, [identifier, password, onLoginSuccess]);

  return {
    identifier,
    password,
    showPassword,
    fieldErrors,
    formError,
    loading,
    isValid,
    setIdentifier,
    setPassword,
    toggleShowPassword,
    handleSubmit,
    canUseBiometrics,
    biometricLabel,
    biometricLoading: biometrics.isLoading,
    handleBiometricLogin,
  };
}
