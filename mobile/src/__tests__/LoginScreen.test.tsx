import React from 'react';
import {
  render,
  fireEvent,
  waitFor,
  act,
} from '@testing-library/react-native';

import { LoginScreen } from '@components/LoginScreen';
import {
  getBiometricLabel,
  getLoginErrorMessage,
  validateIdentifier,
  validatePassword,
} from '@hooks/useLoginScreen';
import { useAuthStore } from '@store/authStore';

jest.mock('@services/api', () => ({
  apiClient: {
    post: jest.fn(),
  },
}));

jest.mock('@services/tokenStorage', () => ({
  saveToken: jest.fn(() => Promise.resolve()),
  getToken: jest.fn(() => Promise.resolve(null)),
}));

jest.mock('@services/biometricService', () => ({
  isBiometricAvailable: jest.fn(() => Promise.resolve(false)),
  getBiometricType: jest.fn(() => Promise.resolve('None')),
  authenticate: jest.fn(() => Promise.resolve(false)),
}));

import { apiClient } from '@services/api';
import { getToken } from '@services/tokenStorage';
import {
  authenticate as biometricAuthenticate,
  getBiometricType,
  isBiometricAvailable,
} from '@services/biometricService';

const mockPost = apiClient.post as jest.Mock;
const mockGetToken = getToken as jest.Mock;
const mockIsBiometricAvailable = isBiometricAvailable as jest.Mock;
const mockGetBiometricType = getBiometricType as jest.Mock;
const mockBiometricAuthenticate = biometricAuthenticate as jest.Mock;

const SUCCESS_RESPONSE = {
  tokens: {
    accessToken: 'access-token',
    refreshToken: 'refresh-token',
    expiresAt: Date.now() + 3600_000,
  },
  user: { id: 'user-1', publicKey: 'GABC', createdAt: '2024-01-01' },
};

function makeAxiosError(status?: number) {
  return {
    isAxiosError: true,
    response: status ? { status } : undefined,
    message: 'request failed',
  };
}

function fillForm(
  api: ReturnType<typeof render>,
  identifier = 'user@example.com',
  password = 'password123',
) {
  fireEvent.changeText(api.getByLabelText('Email or username'), identifier);
  fireEvent.changeText(api.getByLabelText('Password'), password);
}

describe('useLoginScreen validators', () => {
  it('validates the identifier field', () => {
    expect(validateIdentifier('')).toBe('Email or username is required');
    expect(validateIdentifier('bad@email')).toBe('Enter a valid email address');
    expect(validateIdentifier('ab')).toContain('at least');
    expect(validateIdentifier('validuser')).toBeUndefined();
    expect(validateIdentifier('user@example.com')).toBeUndefined();
  });

  it('validates the password field', () => {
    expect(validatePassword('')).toBe('Password is required');
    expect(validatePassword('123')).toContain('at least');
    expect(validatePassword('longenough')).toBeUndefined();
  });

  it('maps errors to friendly messages', () => {
    expect(getLoginErrorMessage(makeAxiosError())).toContain('Unable to connect');
    expect(getLoginErrorMessage(makeAxiosError(401))).toContain('Incorrect');
    expect(getLoginErrorMessage(makeAxiosError(403))).toContain('Incorrect');
    expect(getLoginErrorMessage(new Error('boom'))).toContain('Something went wrong');
  });

  it('maps biometric types to platform labels', () => {
    expect(getBiometricLabel('FaceID')).toBe('Face ID');
    expect(getBiometricLabel('TouchID')).toBe('Touch ID');
    expect(getBiometricLabel('Fingerprint')).toBe('Fingerprint');
    expect(getBiometricLabel('None')).toBe('Biometrics');
  });
});

describe('LoginScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockGetToken.mockResolvedValue(null);
    mockIsBiometricAvailable.mockResolvedValue(false);
    mockGetBiometricType.mockResolvedValue('None');
    mockBiometricAuthenticate.mockResolvedValue(false);
    act(() => {
      useAuthStore.setState({
        user: null,
        tokens: null,
        isAuthenticated: false,
      });
    });
  });

  it('renders correctly', () => {
    const { getByText, getByLabelText } = render(<LoginScreen />);

    expect(getByText('PayRaider')).toBeTruthy();
    expect(getByLabelText('Email or username')).toBeTruthy();
    expect(getByLabelText('Password')).toBeTruthy();
    expect(getByLabelText('Sign in')).toBeTruthy();
  });

  it('shows validation errors on empty submit', async () => {
    const screen = render(<LoginScreen />);

    fireEvent.press(screen.getByLabelText('Sign in'));

    await waitFor(() => {
      expect(screen.getByText('Email or username is required')).toBeTruthy();
    });
    expect(screen.getByText('Password is required')).toBeTruthy();
    expect(mockPost).not.toHaveBeenCalled();
  });

  it('shows an inline error for an invalid email format', async () => {
    const screen = render(<LoginScreen />);

    fillForm(screen, 'bad@email', 'password123');
    fireEvent.press(screen.getByLabelText('Sign in'));

    await waitFor(() => {
      expect(screen.getByText('Enter a valid email address')).toBeTruthy();
    });
    expect(mockPost).not.toHaveBeenCalled();
  });

  it('toggles password visibility', () => {
    const screen = render(<LoginScreen />);
    const passwordInput = screen.getByLabelText('Password');

    expect(passwordInput.props.secureTextEntry).toBe(true);

    fireEvent.press(screen.getByLabelText('Show password'));
    expect(passwordInput.props.secureTextEntry).toBe(false);
    expect(screen.getByLabelText('Hide password')).toBeTruthy();
  });

  it('submits credentials and authenticates on success', async () => {
    mockPost.mockResolvedValue(SUCCESS_RESPONSE);
    const onLoginSuccess = jest.fn();

    const screen = render(<LoginScreen onLoginSuccess={onLoginSuccess} />);
    fillForm(screen, 'user@example.com', 'password123');
    fireEvent.press(screen.getByLabelText('Sign in'));

    await waitFor(() => {
      expect(onLoginSuccess).toHaveBeenCalledTimes(1);
    });

    expect(mockPost).toHaveBeenCalledWith('/auth/login', {
      identifier: 'user@example.com',
      password: 'password123',
    });
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
    expect(useAuthStore.getState().tokens).toEqual(SUCCESS_RESPONSE.tokens);
  });

  it('shows a loading state while the request is in flight', async () => {
    let resolveRequest: (value: unknown) => void = () => {};
    mockPost.mockReturnValue(
      new Promise(resolve => {
        resolveRequest = resolve;
      }),
    );

    const screen = render(<LoginScreen />);
    fillForm(screen);
    fireEvent.press(screen.getByLabelText('Sign in'));

    await waitFor(() => {
      expect(screen.getByLabelText('Sign in').props.accessibilityState.busy).toBe(
        true,
      );
    });

    await act(async () => {
      resolveRequest(SUCCESS_RESPONSE);
    });
  });

  it('shows a global error banner on wrong credentials', async () => {
    mockPost.mockRejectedValue(makeAxiosError(401));

    const screen = render(<LoginScreen />);
    fillForm(screen);
    fireEvent.press(screen.getByLabelText('Sign in'));

    await waitFor(() => {
      expect(
        screen.getByText('Incorrect email/username or password.'),
      ).toBeTruthy();
    });
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
  });

  it('shows a network error banner when the request fails to reach the server', async () => {
    mockPost.mockRejectedValue(makeAxiosError());

    const screen = render(<LoginScreen />);
    fillForm(screen);
    fireEvent.press(screen.getByLabelText('Sign in'));

    await waitFor(() => {
      expect(
        screen.getByText(
          'Unable to connect. Check your internet connection and try again.',
        ),
      ).toBeTruthy();
    });
  });

  it('hides the biometric button when biometrics are unavailable', async () => {
    const screen = render(<LoginScreen />);
    await waitFor(() => expect(mockGetToken).toHaveBeenCalled());
    expect(screen.queryByLabelText(/Sign in with/)).toBeNull();
  });

  it('shows the biometric button and signs in a returning user', async () => {
    mockIsBiometricAvailable.mockResolvedValue(true);
    mockGetBiometricType.mockResolvedValue('FaceID');
    mockGetToken.mockResolvedValue('stored-token');
    mockBiometricAuthenticate.mockResolvedValue(true);
    const onLoginSuccess = jest.fn();

    const screen = render(<LoginScreen onLoginSuccess={onLoginSuccess} />);
    const button = await screen.findByLabelText('Sign in with Face ID');

    fireEvent.press(button);

    await waitFor(() => expect(onLoginSuccess).toHaveBeenCalledTimes(1));
    expect(mockBiometricAuthenticate).toHaveBeenCalledWith(
      'Sign in with Face ID',
    );
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
  });

  it('falls back to a non-alarming message when biometrics are canceled', async () => {
    mockIsBiometricAvailable.mockResolvedValue(true);
    mockGetBiometricType.mockResolvedValue('TouchID');
    mockGetToken.mockResolvedValue('stored-token');
    mockBiometricAuthenticate.mockResolvedValue(false);

    const screen = render(<LoginScreen />);
    const button = await screen.findByLabelText('Sign in with Touch ID');

    fireEvent.press(button);

    await waitFor(() => {
      expect(
        screen.getByText(/Touch ID sign-in didn’t complete/),
      ).toBeTruthy();
    });
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
  });

  it('still authenticates when secure token persistence fails', async () => {
    mockPost.mockResolvedValue(SUCCESS_RESPONSE);
    const { saveToken } = jest.requireMock('@services/tokenStorage') as {
      saveToken: jest.Mock;
    };
    saveToken.mockRejectedValueOnce(new Error('keychain unavailable'));
    const onLoginSuccess = jest.fn();

    const screen = render(<LoginScreen onLoginSuccess={onLoginSuccess} />);
    fillForm(screen);
    fireEvent.press(screen.getByLabelText('Sign in'));

    await waitFor(() => expect(onLoginSuccess).toHaveBeenCalledTimes(1));
    expect(useAuthStore.getState().isAuthenticated).toBe(true);
  });

  it('shows a fallback message when the stored token is gone at biometric time', async () => {
    mockIsBiometricAvailable.mockResolvedValue(true);
    mockGetBiometricType.mockResolvedValue('FaceID');
    mockGetToken.mockResolvedValueOnce('stored-token').mockResolvedValue(null);
    mockBiometricAuthenticate.mockResolvedValue(true);

    const screen = render(<LoginScreen />);
    const button = await screen.findByLabelText('Sign in with Face ID');

    fireEvent.press(button);

    await waitFor(() => {
      expect(
        screen.getByText(/Your saved sign-in is no longer available/),
      ).toBeTruthy();
    });
    expect(useAuthStore.getState().isAuthenticated).toBe(false);
  });

  it('clears a field error once the user edits the field', async () => {
    const screen = render(<LoginScreen />);

    fireEvent.press(screen.getByLabelText('Sign in'));
    await waitFor(() => {
      expect(screen.getByText('Password is required')).toBeTruthy();
    });

    fireEvent.changeText(screen.getByLabelText('Password'), 'newpassword');
    expect(screen.queryByText('Password is required')).toBeNull();
  });
});
