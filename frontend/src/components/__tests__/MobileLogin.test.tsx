/**
 * Regression tests for MobileLogin component post-update biometric recovery UI.
 *
 * Covers:
 *   - Re-registration banner shown when credential is stale (post-update)
 *   - Re-registration banner shown on STALE_CREDENTIAL / SECURITY_ERROR
 *   - Normal authenticate flow when credential is current
 *   - First-time registration flow when not enrolled
 *   - Fallback button rendered when onFallback is provided
 *   - Success state renders correctly
 *   - Not-supported state renders correctly
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MobileLogin } from '../MobileLogin';
import * as useBiometricAuthModule from '@/hooks/useBiometricAuth';
import type { UseBiometricAuthReturn } from '@/hooks/useBiometricAuth';

vi.mock('@/hooks/useBiometricAuth');

const mockHook = useBiometricAuthModule.useBiometricAuth as ReturnType<typeof vi.fn>;

function baseHook(overrides: Partial<UseBiometricAuthReturn> = {}): UseBiometricAuthReturn {
  return {
    state: 'idle',
    error: null,
    isSupported: true,
    isEnrolled: true,
    isStale: false,
    credential: null,
    authenticate: vi.fn().mockResolvedValue(true),
    register: vi.fn().mockResolvedValue(true),
    reset: vi.fn(),
    ...overrides,
  };
}

const defaultProps = {
  userId: 'user@example.com',
  displayName: 'Test User',
  onSuccess: vi.fn(),
  onFallback: vi.fn(),
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe('MobileLogin — post-update re-registration flow', () => {
  it('shows re-registration UI when isStale is true', () => {
    mockHook.mockReturnValue(baseHook({ isStale: true }));
    render(<MobileLogin {...defaultProps} />);
    expect(screen.getByText(/App update detected/i)).toBeTruthy();
    expect(screen.getByRole('button', { name: /Re-register Biometric/i })).toBeTruthy();
  });

  it('shows re-registration UI on STALE_CREDENTIAL error', () => {
    mockHook.mockReturnValue(baseHook({ error: 'STALE_CREDENTIAL' }));
    render(<MobileLogin {...defaultProps} />);
    expect(screen.getByText(/App update detected/i)).toBeTruthy();
  });

  it('shows re-registration UI on SECURITY_ERROR', () => {
    mockHook.mockReturnValue(baseHook({ error: 'SECURITY_ERROR' }));
    render(<MobileLogin {...defaultProps} />);
    expect(screen.getByText(/App update detected/i)).toBeTruthy();
  });

  it('calls register and onSuccess when re-registration succeeds', async () => {
    const register = vi.fn().mockResolvedValue(true);
    mockHook.mockReturnValue(baseHook({ isStale: true, register }));
    render(<MobileLogin {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: /Re-register Biometric/i }));
    await waitFor(() => {
      expect(register).toHaveBeenCalledWith('user@example.com', 'Test User');
      expect(defaultProps.onSuccess).toHaveBeenCalledWith('user@example.com');
    });
  });
});

describe('MobileLogin — normal authentication flow', () => {
  it('shows authenticate button when enrolled and not stale', () => {
    mockHook.mockReturnValue(baseHook());
    render(<MobileLogin {...defaultProps} />);
    expect(screen.getByRole('button', { name: /Authenticate/i })).toBeTruthy();
  });

  it('calls authenticate and onSuccess on success', async () => {
    const authenticate = vi.fn().mockResolvedValue(true);
    mockHook.mockReturnValue(baseHook({ authenticate }));
    render(<MobileLogin {...defaultProps} />);
    fireEvent.click(screen.getByRole('button', { name: /Authenticate/i }));
    await waitFor(() => {
      expect(authenticate).toHaveBeenCalledWith('user@example.com');
      expect(defaultProps.onSuccess).toHaveBeenCalledWith('user@example.com');
    });
  });

  it('shows error banner on auth failure', () => {
    mockHook.mockReturnValue(baseHook({ error: 'USER_CANCELLED', state: 'error' }));
    render(<MobileLogin {...defaultProps} />);
    expect(screen.getByText(/cancelled/i)).toBeTruthy();
  });
});

describe('MobileLogin — first-time registration', () => {
  it('shows set-up button when not enrolled', () => {
    mockHook.mockReturnValue(baseHook({ isEnrolled: false }));
    render(<MobileLogin {...defaultProps} />);
    expect(screen.getByRole('button', { name: /Set Up Biometric Login/i })).toBeTruthy();
  });
});

describe('MobileLogin — edge states', () => {
  it('shows not-supported message when biometrics unavailable', () => {
    mockHook.mockReturnValue(baseHook({ isSupported: false, state: 'idle' }));
    render(<MobileLogin {...defaultProps} />);
    expect(screen.getByText(/not available/i)).toBeTruthy();
  });

  it('shows success state after authentication', () => {
    mockHook.mockReturnValue(baseHook({ state: 'success' }));
    render(<MobileLogin {...defaultProps} />);
    expect(screen.getByText(/Authenticated successfully/i)).toBeTruthy();
  });

  it('renders fallback button when onFallback is provided', () => {
    mockHook.mockReturnValue(baseHook());
    render(<MobileLogin {...defaultProps} />);
    expect(screen.getByRole('button', { name: /Use another login method/i })).toBeTruthy();
  });
});
