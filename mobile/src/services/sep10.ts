/**
 * SEP-10 mobile authentication client for PayRaider.
 *
 * SEP-10 is the Stellar Ecosystem Proposal that defines a standard challenge/
 * response authentication protocol for Stellar-based services.  The backend
 * exposes four endpoints (see `backend/src/api/sep10.rs`):
 *
 *   GET  /api/sep10/info    — server info (signing key, network passphrase)
 *   POST /api/sep10/auth    — request a challenge transaction
 *   POST /api/sep10/verify  — submit a signed challenge to obtain a session token
 *   POST /api/sep10/logout  — invalidate a session token
 *
 * ## Usage
 *
 * ```ts
 * // 1. Get server info (optional — useful to display network to the user)
 * const info = await sep10.getServerInfo();
 *
 * // 2. Request a challenge for the user's Stellar account
 * const challenge = await sep10.requestChallenge({
 *   account: 'G...',
 *   home_domain: 'payraider.com',
 * });
 *
 * // 3. Sign challenge.transaction with the account's private key (app-specific)
 * const signedTransaction = myWallet.sign(challenge.transaction);
 *
 * // 4. Verify the signed challenge — the backend returns a session token
 * const session = await sep10.verifyChallenge(signedTransaction);
 * // session.token is now stored via storeAuthTokens and the auth store is updated
 *
 * // 5. On logout
 * await sep10.logout();
 * ```
 *
 * ## Integration with the auth store
 * `verifyChallenge` converts the SEP-10 session token into an `AuthTokens`
 * object compatible with the existing `api.ts` interceptor (Bearer header) and
 * `auth.ts` persistence layer.
 */

import { apiClient } from './api';
import { storeAuthTokens, clearAuthTokens } from './auth';
import type { AuthTokens } from '@app-types/index';

// ---------------------------------------------------------------------------
// Request / Response shapes (mirror backend/src/auth/sep10_simple.rs)
// ---------------------------------------------------------------------------

/** Body sent to `POST /api/sep10/auth`. */
export interface Sep10ChallengeRequest {
  /** Stellar account public key (starts with G, 56 chars). */
  account: string;
  /** Optional — anchor's home domain. */
  home_domain?: string;
  /** Optional — client application domain. */
  client_domain?: string;
  /** Optional — account memo (string representation of a uint64). */
  memo?: string;
}

/** Response from `POST /api/sep10/auth`. */
export interface Sep10ChallengeResponse {
  /** Base64-encoded challenge transaction (XDR or simplified JSON-in-base64). */
  transaction: string;
  /** Stellar network passphrase used by the server. */
  network_passphrase: string;
}

/** Response from `POST /api/sep10/verify`. */
export interface Sep10VerificationResponse {
  /** Session bearer token. Store and attach to subsequent API requests. */
  token: string;
  /** Session lifetime in seconds from issue time. */
  expires_in: number;
}

/** Response from `GET /api/sep10/info`. */
export interface Sep10ServerInfo {
  /** URL of the SEP-10 authentication endpoint. */
  authentication_endpoint: string;
  /** Stellar network passphrase. */
  network_passphrase: string;
  /** Server's Stellar public key used to sign challenges. */
  signing_key: string;
  /** SEP-10 protocol version supported by this server. */
  version: string;
}

/** Structured error thrown by this service. */
export class Sep10Error extends Error {
  /** HTTP status code, if the error originated from an API response. */
  readonly status?: number;

  constructor(message: string, status?: number) {
    super(message);
    this.name = 'Sep10Error';
    this.status = status;
  }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/**
 * Fetch SEP-10 server information (signing key, network passphrase, endpoint).
 *
 * This is optional for the auth flow but useful for displaying network context
 * to the user or validating that the server key matches a known value.
 *
 * @returns {@link Sep10ServerInfo}
 * @throws {@link Sep10Error} on network or API failure.
 */
export async function getServerInfo(): Promise<Sep10ServerInfo> {
  try {
    return await apiClient.get<Sep10ServerInfo>('/api/sep10/info');
  } catch (error) {
    throw normaliseSep10Error(error, 'Failed to fetch SEP-10 server info');
  }
}

/**
 * Request a SEP-10 challenge transaction from the server.
 *
 * The returned `transaction` field must be signed by the account's private key
 * and then submitted to {@link verifyChallenge}.
 *
 * @param request - Challenge request params (account is required).
 * @returns {@link Sep10ChallengeResponse}
 * @throws {@link Sep10Error} on validation or network failure.
 */
export async function requestChallenge(
  request: Sep10ChallengeRequest,
): Promise<Sep10ChallengeResponse> {
  if (!request.account.startsWith('G') || request.account.length !== 56) {
    throw new Sep10Error(
      'Invalid Stellar account: must start with G and be 56 characters long.',
    );
  }

  try {
    return await apiClient.post<Sep10ChallengeResponse>('/api/sep10/auth', request);
  } catch (error) {
    throw normaliseSep10Error(error, 'Failed to request SEP-10 challenge');
  }
}

/**
 * Submit a signed challenge transaction to the server and obtain a session token.
 *
 * On success:
 *  - Converts the session token into an `AuthTokens` object.
 *  - Persists the tokens via `storeAuthTokens` (react-native-keychain).
 *  - Updates the auth store (`isAuthenticated: true`, tokens set).
 *
 * @param signedTransaction - Base64-encoded signed challenge (XDR or JSON-in-base64).
 * @returns The raw {@link Sep10VerificationResponse} (token + expires_in).
 * @throws {@link Sep10Error} on signature verification failure, expiry, or network error.
 */
export async function verifyChallenge(
  signedTransaction: string,
): Promise<Sep10VerificationResponse> {
  if (!signedTransaction) {
    throw new Sep10Error('Signed transaction must not be empty.');
  }

  let response: Sep10VerificationResponse;
  try {
    response = await apiClient.post<Sep10VerificationResponse>('/api/sep10/verify', {
      transaction: signedTransaction,
    });
  } catch (error) {
    throw normaliseSep10Error(error, 'SEP-10 challenge verification failed');
  }

  // Convert SEP-10 session into the AuthTokens shape the rest of the app uses.
  // SEP-10 issues a single opaque bearer token; we store it as the accessToken.
  // expiresAt is calculated from expires_in (seconds → ms epoch).
  const expiresAt = Date.now() + response.expires_in * 1000;
  const tokens: AuthTokens = {
    accessToken: response.token,
    // SEP-10 does not issue a refresh token; set to empty string so the type
    // is satisfied.  The interceptor in api.ts will treat an empty refreshToken
    // as a sign-in required condition.
    refreshToken: '',
    expiresAt,
  };

  await storeAuthTokens(tokens);

  return response;
}

/**
 * Invalidate the current SEP-10 session on the server and clear local auth
 * state.
 *
 * This is a best-effort operation: local state is cleared regardless of
 * whether the server request succeeds, so the user is always signed out
 * locally even if the server is unreachable.
 *
 * @throws {@link Sep10Error} only when the local clear itself fails (rare).
 */
export async function logout(): Promise<void> {
  // Best-effort server-side invalidation.
  try {
    await apiClient.post<void>('/api/sep10/logout', {});
  } catch (error) {
    // Log but do not rethrow — local logout must still proceed.
    console.warn('SEP-10 server logout failed (local session will still be cleared):', error);
  }

  // Always clear local tokens and auth store.
  await clearAuthTokens();
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Normalise any thrown value into a {@link Sep10Error}.
 * Extracts HTTP status and server error message when available.
 */
function normaliseSep10Error(error: unknown, fallbackMessage: string): Sep10Error {
  if (error instanceof Sep10Error) return error;

  // axios errors carry response.status and response.data.error
  if (
    error !== null &&
    typeof error === 'object' &&
    'response' in error
  ) {
    const axiosError = error as {
      response?: { status?: number; data?: { error?: string } };
      message?: string;
    };
    const status = axiosError.response?.status;
    const serverMessage = axiosError.response?.data?.error;
    return new Sep10Error(
      serverMessage ?? axiosError.message ?? fallbackMessage,
      status,
    );
  }

  if (error instanceof Error) {
    return new Sep10Error(error.message, undefined);
  }

  return new Sep10Error(fallbackMessage);
}
