'use client';

import { useState } from 'react';
import { Fingerprint, RefreshCw, ShieldAlert, CheckCircle, Loader2, XCircle } from 'lucide-react';
import { useBiometricAuth } from '@/hooks/useBiometricAuth';
import type { BiometricError } from '@/services/biometricAuth';

interface MobileLoginProps {
  userId: string;
  displayName: string;
  onSuccess: (userId: string) => void;
  onFallback?: () => void;
}

const ERROR_MESSAGES: Record<BiometricError, string> = {
  NOT_SUPPORTED: 'Biometric authentication is not supported on this device.',
  NOT_ENROLLED: 'No biometric credential found. Please register first.',
  STALE_CREDENTIAL:
    'Your biometric credential needs to be updated after the recent app update. Please re-register.',
  USER_CANCELLED: 'Authentication was cancelled. Please try again.',
  INVALID_STATE: 'A credential is already registered for this device.',
  SECURITY_ERROR:
    'A security error occurred. This can happen after an app update, domain change, or deployment — please re-register.',
  UNKNOWN: 'An unexpected error occurred. Please try again.',
};

export function MobileLogin({ userId, displayName, onSuccess, onFallback }: MobileLoginProps) {
  const { state, error, isSupported, isEnrolled, isStale, authenticate, register, reset } =
    useBiometricAuth(userId);
  const [showReRegister, setShowReRegister] = useState(false);

  const needsReRegistration =
    isStale || error === 'STALE_CREDENTIAL' || error === 'SECURITY_ERROR';

  async function handleAuthenticate() {
    const ok = await authenticate(userId);
    if (ok) onSuccess(userId);
  }

  async function handleRegister() {
    const ok = await register(userId, displayName);
    if (ok) {
      setShowReRegister(false);
      onSuccess(userId);
    }
  }

  function handleReset() {
    reset(userId);
    setShowReRegister(false);
  }

  const isLoading = state === 'authenticating' || state === 'registering' || state === 'checking';

  if (state === 'checking') {
    return (
      <div className="flex flex-col items-center justify-center min-h-[200px] gap-3">
        <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
        <p className="text-sm text-muted-foreground">Checking biometric availability…</p>
      </div>
    );
  }

  if (!isSupported) {
    return (
      <div className="flex flex-col items-center gap-4 p-6 text-center">
        <ShieldAlert className="w-12 h-12 text-yellow-500" />
        <p className="text-sm text-muted-foreground">
          Biometric authentication is not available on this device or browser.
        </p>
        {onFallback && (
          <button
            onClick={onFallback}
            className="w-full py-3 px-6 rounded-xl bg-blue-600 hover:bg-blue-700 text-white font-semibold transition"
          >
            Use another login method
          </button>
        )}
      </div>
    );
  }

  if (state === 'success') {
    return (
      <div className="flex flex-col items-center gap-3 p-6 text-center">
        <CheckCircle className="w-12 h-12 text-green-500" />
        <p className="font-semibold text-gray-900 dark:text-white">Authenticated successfully</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5 p-6">
      <div className="text-center">
        <Fingerprint className="w-14 h-14 mx-auto text-blue-500 mb-2" />
        <h2 className="text-xl font-bold text-gray-900 dark:text-white">Biometric Login</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Use Face ID, Touch ID, or your device fingerprint sensor
        </p>
      </div>

      {/* Error banner */}
      {error && (
        <div className="flex items-start gap-2 rounded-xl border border-red-300 bg-red-50 dark:bg-red-900/20 p-3">
          <XCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
          <p className="text-sm text-red-700 dark:text-red-300">{ERROR_MESSAGES[error]}</p>
        </div>
      )}

      {/* Re-registration flow (post-update recovery) */}
      {(needsReRegistration || showReRegister) ? (
        <div className="flex flex-col gap-3">
          <div className="rounded-xl border border-yellow-300 bg-yellow-50 dark:bg-yellow-900/20 p-3 text-sm text-yellow-800 dark:text-yellow-200">
            <strong>App update detected.</strong> Your saved biometric credential needs to be
            refreshed. This is a one-time step.
          </div>
          <button
            onClick={handleRegister}
            disabled={isLoading}
            className="w-full flex items-center justify-center gap-2 py-3 px-6 rounded-xl bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white font-semibold transition"
          >
            {state === 'registering' ? (
              <Loader2 className="w-5 h-5 animate-spin" />
            ) : (
              <RefreshCw className="w-5 h-5" />
            )}
            Re-register Biometric
          </button>
          <button
            onClick={handleReset}
            className="w-full py-2 text-sm text-muted-foreground hover:text-gray-700 dark:hover:text-gray-300 transition"
          >
            Cancel
          </button>
        </div>
      ) : isEnrolled ? (
        /* Normal authentication */
        <div className="flex flex-col gap-3">
          <button
            onClick={handleAuthenticate}
            disabled={isLoading}
            className="w-full flex items-center justify-center gap-2 py-3 px-6 rounded-xl bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white font-semibold transition"
          >
            {state === 'authenticating' ? (
              <Loader2 className="w-5 h-5 animate-spin" />
            ) : (
              <Fingerprint className="w-5 h-5" />
            )}
            Authenticate
          </button>
          <button
            onClick={() => setShowReRegister(true)}
            className="w-full py-2 text-sm text-muted-foreground hover:text-gray-700 dark:hover:text-gray-300 transition"
          >
            Having trouble? Re-register biometric
          </button>
        </div>
      ) : (
        /* First-time registration */
        <button
          onClick={handleRegister}
          disabled={isLoading}
          className="w-full flex items-center justify-center gap-2 py-3 px-6 rounded-xl bg-green-600 hover:bg-green-700 disabled:opacity-50 text-white font-semibold transition"
        >
          {state === 'registering' ? (
            <Loader2 className="w-5 h-5 animate-spin" />
          ) : (
            <Fingerprint className="w-5 h-5" />
          )}
          Set Up Biometric Login
        </button>
      )}

      {onFallback && (
        <button
          onClick={onFallback}
          className="w-full py-2 text-sm text-center text-muted-foreground hover:text-gray-700 dark:hover:text-gray-300 transition"
        >
          Use another login method
        </button>
      )}
    </div>
  );
}
