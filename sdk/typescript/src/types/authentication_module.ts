import type { AuthTokens } from "../types.js";

export type AuthenticationAction =
  | "login"
  | "refresh"
  | "logout"
  | "getSession"
  | "sep10Challenge"
  | "sep10Verify";

export interface AuthenticationModuleParams {
  action: AuthenticationAction;
  email?: string;
  password?: string;
  refreshToken?: string;
  autoRefresh?: boolean;
  account?: string;
  homeDomain?: string;
  clientDomain?: string;
  memo?: string;
  /** Base64-encoded signed SEP-10 transaction XDR */
  transaction?: string;
}

export interface StoredAuthSession {
  accessToken: string;
  refreshToken?: string;
  expiresAt: number;
  authMethod: "credentials" | "sep10";
  account?: string;
}

export interface TokenStorage {
  getSession(): Promise<StoredAuthSession | null>;
  setSession(session: StoredAuthSession): Promise<void>;
  clearSession(): Promise<void>;
}

export interface AuthenticationModuleResult {
  success: true;
  action: AuthenticationAction;
  authenticated: boolean;
  data: {
    executedAt: string;
    session?: StoredAuthSession;
    challenge?: {
      transaction: string;
      networkPassphrase: string;
    };
    tokens?: AuthTokens | { token: string; expires_in: number };
  };
}

export type AuthEventType = "login" | "logout" | "refresh" | "sep10" | "error";

export type AuthEventListener = (
  event: AuthEventType,
  payload?: Record<string, unknown>,
) => void;
