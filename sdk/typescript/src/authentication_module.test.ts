import { describe, expect, it, vi, beforeEach } from "vitest";
import {
  AuthenticationModule,
  SDKError,
  type StoredAuthSession,
  type TokenStorage,
} from "../src/index.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

function ok(body: unknown) {
  return Promise.resolve({
    ok: true,
    status: 200,
    json: () => Promise.resolve(body),
    headers: { get: () => null },
  });
}

function err(status: number, body: unknown) {
  return Promise.resolve({
    ok: false,
    status,
    statusText: "Error",
    json: () => Promise.resolve(body),
    headers: { get: () => null },
  });
}

class TestTokenStorage implements TokenStorage {
  session: StoredAuthSession | null = null;

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

describe("AuthenticationModule", () => {
  let auth: AuthenticationModule;
  let storage: TestTokenStorage;

  beforeEach(() => {
    mockFetch.mockReset();
    storage = new TestTokenStorage();
    auth = new AuthenticationModule({ apiKey: "test-key", maxRetries: 0 }, storage);
  });

  // ── execute ────────────────────────────────────────────────────────────────

  it("should execute successfully on login", async () => {
    mockFetch.mockReturnValueOnce(
      ok({
        access_token: "access-1",
        refresh_token: "refresh-1",
        expires_in: 3600,
      }),
    );

    const result = await auth.execute({
      action: "login",
      email: "user@example.com",
      password: "secret",
    });

    expect(result.success).toBe(true);
    expect(result.action).toBe("login");
    expect(result.authenticated).toBe(true);
    expect(result.data.session?.accessToken).toBe("access-1");
    expect(await auth.isAuthenticated()).toBe(true);
  });

  it("refreshes tokens and updates storage", async () => {
    storage.session = {
      accessToken: "old-access",
      refreshToken: "refresh-1",
      expiresAt: Date.now() + 3600_000,
      authMethod: "credentials",
    };

    mockFetch.mockReturnValueOnce(
      ok({
        access_token: "new-access",
        refresh_token: "refresh-2",
        expires_in: 3600,
      }),
    );

    const result = await auth.execute({ action: "refresh" });

    expect(result.action).toBe("refresh");
    expect(result.data.session?.accessToken).toBe("new-access");
    expect(storage.session?.accessToken).toBe("new-access");
  });

  it("logs out and clears the stored session", async () => {
    storage.session = {
      accessToken: "access-1",
      refreshToken: "refresh-1",
      expiresAt: Date.now() + 3600_000,
      authMethod: "credentials",
    };

    mockFetch.mockReturnValueOnce(ok({}));

    const result = await auth.execute({ action: "logout" });

    expect(result.authenticated).toBe(false);
    expect(storage.session).toBeNull();
    expect(await auth.isAuthenticated()).toBe(false);
  });

  it("returns current session via getSession", async () => {
    storage.session = {
      accessToken: "access-1",
      refreshToken: "refresh-1",
      expiresAt: Date.now() + 3600_000,
      authMethod: "credentials",
    };

    const result = await auth.execute({ action: "getSession", autoRefresh: false });

    expect(result.authenticated).toBe(true);
    expect(result.data.session?.accessToken).toBe("access-1");
  });

  it("auto-refreshes an expiring session during getSession", async () => {
    storage.session = {
      accessToken: "old-access",
      refreshToken: "refresh-1",
      expiresAt: Date.now() + 30_000,
      authMethod: "credentials",
    };

    mockFetch.mockReturnValueOnce(
      ok({
        access_token: "fresh-access",
        refresh_token: "refresh-1",
        expires_in: 3600,
      }),
    );

    const result = await auth.execute({ action: "getSession", autoRefresh: true });

    expect(result.action).toBe("refresh");
    expect(result.data.session?.accessToken).toBe("fresh-access");
  });

  it("requests a SEP-10 challenge", async () => {
    mockFetch.mockReturnValueOnce(
      ok({
        transaction: "challenge-xdr",
        network_passphrase: "Public Global Stellar Network ; September 2015",
      }),
    );

    const result = await auth.execute({
      action: "sep10Challenge",
      account: "GABC123",
    });

    expect(result.action).toBe("sep10Challenge");
    expect(result.data.challenge?.transaction).toBe("challenge-xdr");
  });

  it("verifies a signed SEP-10 transaction", async () => {
    mockFetch.mockReturnValueOnce(
      ok({
        token: "sep10-token",
        expires_in: 604800,
      }),
    );

    const result = await auth.execute({
      action: "sep10Verify",
      transaction: "signed-xdr",
    });

    expect(result.action).toBe("sep10Verify");
    expect(result.authenticated).toBe(true);
    expect(storage.session?.authMethod).toBe("sep10");
    expect(storage.session?.accessToken).toBe("sep10-token");
  });

  it("throws SDKError when login email is missing", async () => {
    await expect(
      auth.execute({ action: "login", password: "secret" }),
    ).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when sep10Challenge account is missing", async () => {
    await expect(auth.execute({ action: "sep10Challenge" })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when refresh token is unavailable", async () => {
    await expect(auth.execute({ action: "refresh" })).rejects.toBeInstanceOf(SDKError);
  });

  // ── events ─────────────────────────────────────────────────────────────────

  it("emits login events", async () => {
    const listener = vi.fn();
    auth.onAuthEvent(listener);

    mockFetch.mockReturnValueOnce(
      ok({
        access_token: "access-1",
        refresh_token: "refresh-1",
        expires_in: 3600,
      }),
    );

    await auth.execute({
      action: "login",
      email: "user@example.com",
      password: "secret",
    });

    expect(listener).toHaveBeenCalledWith("login", { authMethod: "credentials" });
  });

  it("unsubscribe stops auth event notifications", async () => {
    const listener = vi.fn();
    const unsubscribe = auth.onAuthEvent(listener);
    unsubscribe();

    mockFetch.mockReturnValueOnce(
      ok({
        access_token: "access-1",
        refresh_token: "refresh-1",
        expires_in: 3600,
      }),
    );

    await auth.execute({
      action: "login",
      email: "user@example.com",
      password: "secret",
    });

    expect(listener).not.toHaveBeenCalled();
  });

  it("throws SDKError when listener is not a function", () => {
    expect(() => auth.onAuthEvent(undefined as unknown as () => void)).toThrow(SDKError);
  });

  // ── helpers ────────────────────────────────────────────────────────────────

  it("getAccessToken returns the active access token", async () => {
    storage.session = {
      accessToken: "access-1",
      expiresAt: Date.now() + 3600_000,
      authMethod: "sep10",
    };

    await expect(auth.getAccessToken(false)).resolves.toBe("access-1");
  });

  it("clears expired sessions during getSession", async () => {
    storage.session = {
      accessToken: "expired",
      expiresAt: Date.now() - 1000,
      authMethod: "credentials",
    };

    const result = await auth.execute({ action: "getSession", autoRefresh: false });

    expect(result.authenticated).toBe(false);
    expect(storage.session).toBeNull();
  });

  it("works with no config argument", async () => {
    const instance = new AuthenticationModule();
    mockFetch.mockReturnValueOnce(
      ok({
        access_token: "access-1",
        refresh_token: "refresh-1",
        expires_in: 3600,
      }),
    );

    const result = await instance.execute({
      action: "login",
      email: "user@example.com",
      password: "secret",
    });

    expect(result.success).toBe(true);
  });

  it("wraps API failures in SDKError", async () => {
    mockFetch.mockReturnValueOnce(
      err(401, { error: "UNAUTHORIZED", message: "Invalid credentials", status: 401 }),
    );

    await expect(
      auth.execute({
        action: "login",
        email: "user@example.com",
        password: "wrong",
      }),
    ).rejects.toBeInstanceOf(SDKError);
  });
});
