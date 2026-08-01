import { HttpClient } from "./http.js";
import type { StellarInsightsConfig, AuthTokens } from "./types.js";
import type {
  AuthEventListener,
  AuthEventType,
  AuthenticationModuleParams,
  AuthenticationModuleResult,
  StoredAuthSession,
  TokenStorage,
} from "./types/authentication_module.js";
import { SDKError } from "./sdk_error.js";

const REFRESH_BUFFER_MS = 60_000;

class InMemoryTokenStorage implements TokenStorage {
  private session: StoredAuthSession | null = null;

  async getSession(): Promise<StoredAuthSession | null> {
    return this.session;
  }

  async setSession(session: StoredAuthSession): Promise<void> {
    this.session = session;
  }

  async clearSession(): Promise<void> {
    this.session = null;
  }
}

export class AuthenticationModule {
  private readonly config: StellarInsightsConfig;
  private readonly http: HttpClient;
  private readonly storage: TokenStorage;
  private readonly listeners = new Set<AuthEventListener>();

  constructor(config: StellarInsightsConfig = {}, storage?: TokenStorage) {
    this.config = config;
    this.http = new HttpClient(config);
    this.storage = storage ?? new InMemoryTokenStorage();
  }

  public async execute(params: AuthenticationModuleParams): Promise<AuthenticationModuleResult> {
    this.validateParams(params);

    try {
      switch (params.action) {
        case "login":
          return await this.login(params.email!.trim(), params.password!);
        case "refresh":
          return await this.refresh(params.refreshToken);
        case "logout":
          return await this.logout();
        case "getSession":
          return await this.getSession(params.autoRefresh ?? true);
        case "sep10Challenge":
          return await this.sep10Challenge(params);
        case "sep10Verify":
          return await this.sep10Verify(params.transaction!.trim());
        default:
          throw new SDKError("Unsupported authentication action", { params });
      }
    } catch (error) {
      this.emit("error", { action: params.action, error });
      if (error instanceof SDKError) throw error;
      throw new SDKError("Failed to execute authentication module", { params }, { cause: error });
    }
  }

  public async getAccessToken(autoRefresh = true): Promise<string | undefined> {
    const result = await this.getSession(autoRefresh);
    return result.data.session?.accessToken;
  }

  public async isAuthenticated(): Promise<boolean> {
    const session = await this.storage.getSession();
    return session !== null && session.expiresAt > Date.now();
  }

  public onAuthEvent(listener: AuthEventListener): () => void {
    if (typeof listener !== "function") {
      throw new SDKError("listener must be a function");
    }

    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private async login(email: string, password: string): Promise<AuthenticationModuleResult> {
    const tokens = await this.http.request<AuthTokens>("POST", "/api/auth/login", {
      body: { email, password },
    });

    const session = this.toCredentialSession(tokens);
    await this.persistSession(session);
    this.http.setToken(session.accessToken);
    this.emit("login", { authMethod: "credentials" });

    return this.buildResult("login", true, { session, tokens });
  }

  private async refresh(refreshToken?: string): Promise<AuthenticationModuleResult> {
    const stored = await this.storage.getSession();
    const token = refreshToken?.trim() ?? stored?.refreshToken;

    if (!token) {
      throw new SDKError("refreshToken is required when no stored session exists");
    }

    const tokens = await this.http.request<AuthTokens>("POST", "/api/auth/refresh", {
      body: { refresh_token: token },
    });

    const session = this.toCredentialSession(tokens, stored?.authMethod ?? "credentials", stored?.account);
    await this.persistSession(session);
    this.http.setToken(session.accessToken);
    this.emit("refresh", { authMethod: session.authMethod });

    return this.buildResult("refresh", true, { session, tokens });
  }

  private async logout(): Promise<AuthenticationModuleResult> {
    try {
      await this.http.request("POST", "/api/auth/logout");
    } catch {
      // Clear local session even if remote logout fails
    }

    await this.storage.clearSession();
    this.http.setToken("");
    this.emit("logout");

    return this.buildResult("logout", false);
  }

  private async getSession(autoRefresh: boolean): Promise<AuthenticationModuleResult> {
    let session = await this.storage.getSession();

    if (!session) {
      return this.buildResult("getSession", false);
    }

    const expiresSoon = session.expiresAt - Date.now() <= REFRESH_BUFFER_MS;
    if (autoRefresh && expiresSoon && session.refreshToken) {
      return this.refresh(session.refreshToken);
    }

    if (session.expiresAt <= Date.now()) {
      await this.storage.clearSession();
      return this.buildResult("getSession", false);
    }

    this.http.setToken(session.accessToken);
    return this.buildResult("getSession", true, { session });
  }

  private async sep10Challenge(
    params: AuthenticationModuleParams,
  ): Promise<AuthenticationModuleResult> {
    const challenge = await this.http.request<{ transaction: string; network_passphrase: string }>(
      "POST",
      "/api/sep10/auth",
      {
        body: {
          account: params.account!.trim(),
          home_domain: params.homeDomain,
          client_domain: params.clientDomain,
          memo: params.memo,
        },
      },
    );

    return this.buildResult("sep10Challenge", false, {
      challenge: {
        transaction: challenge.transaction,
        networkPassphrase: challenge.network_passphrase,
      },
    });
  }

  private async sep10Verify(transaction: string): Promise<AuthenticationModuleResult> {
    const response = await this.http.request<{ token: string; expires_in: number }>(
      "POST",
      "/api/sep10/verify",
      { body: { transaction } },
    );

    const session: StoredAuthSession = {
      accessToken: response.token,
      expiresAt: Date.now() + response.expires_in * 1000,
      authMethod: "sep10",
    };

    await this.persistSession(session);
    this.http.setToken(session.accessToken);
    this.emit("sep10", { authMethod: "sep10" });

    return this.buildResult("sep10Verify", true, {
      session,
      tokens: response,
    });
  }

  private async persistSession(session: StoredAuthSession): Promise<void> {
    await this.storage.setSession(session);
  }

  private toCredentialSession(
    tokens: AuthTokens,
    authMethod: StoredAuthSession["authMethod"] = "credentials",
    account?: string,
  ): StoredAuthSession {
    return {
      accessToken: tokens.access_token,
      refreshToken: tokens.refresh_token,
      expiresAt: Date.now() + tokens.expires_in * 1000,
      authMethod,
      account,
    };
  }

  private buildResult(
    action: AuthenticationModuleParams["action"],
    authenticated: boolean,
    data: Omit<AuthenticationModuleResult["data"], "executedAt"> = {},
  ): AuthenticationModuleResult {
    return {
      success: true,
      action,
      authenticated,
      data: {
        ...data,
        executedAt: new Date().toISOString(),
      },
    };
  }

  private validateParams(params: AuthenticationModuleParams): void {
    if (!params?.action) {
      throw new SDKError("action is required", { params });
    }

    switch (params.action) {
      case "login":
        if (!params.email?.trim() || !params.password) {
          throw new SDKError("email and password are required for login", { params });
        }
        break;
      case "sep10Challenge":
        if (!params.account?.trim()) {
          throw new SDKError("account is required for sep10Challenge", { params });
        }
        break;
      case "sep10Verify":
        if (!params.transaction?.trim()) {
          throw new SDKError("transaction is required for sep10Verify", { params });
        }
        break;
      default:
        break;
    }
  }

  private emit(event: AuthEventType, payload?: Record<string, unknown>): void {
    for (const listener of this.listeners) {
      listener(event, payload);
    }
  }
}
