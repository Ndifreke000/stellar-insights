import type { StellarInsightsConfig, ApiError } from "./types.js";

const DEFAULT_BASE_URL = "https://api.stellarinsights.io";
const DEFAULT_MAX_RETRIES = 3;
const DEFAULT_RETRY_DELAY = 500;
const DEFAULT_TIMEOUT = 30_000;

/** Errors that should trigger a retry */
const RETRYABLE_STATUSES = new Set([429, 500, 502, 503, 504]);

export class StellarInsightsError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
    public readonly requestId?: string,
  ) {
    super(message);
    this.name = "StellarInsightsError";
  }
}

export class HttpClient {
  private readonly baseUrl: string;
  private readonly maxRetries: number;
  private readonly retryDelay: number;
  private readonly timeout: number;
  private token: string | undefined;

  constructor(config: StellarInsightsConfig) {
    this.baseUrl = (config.baseUrl ?? DEFAULT_BASE_URL).replace(/\/$/, "");
    this.maxRetries = config.maxRetries ?? DEFAULT_MAX_RETRIES;
    this.retryDelay = config.retryDelay ?? DEFAULT_RETRY_DELAY;
    this.timeout = config.timeout ?? DEFAULT_TIMEOUT;
    this.token = config.accessToken ?? config.apiKey;
  }

  /** Update the bearer token (e.g. after OAuth login) */
  setToken(token: string): void {
    this.token = token;
  }

  async request<T>(
    method: string,
    path: string,
    options: { body?: unknown; params?: Record<string, unknown>; headers?: Record<string, string> } = {},
  ): Promise<T> {
    const url = this.buildUrl(path, options.params);
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      Accept: "application/json",
    };
    if (this.token) {
      headers["Authorization"] = `Bearer ${this.token}`;
    }
    if (options.headers) {
      Object.assign(headers, options.headers);
    }

    const init: RequestInit = {
      method,
      headers,
      signal: AbortSignal.timeout(this.timeout),
    };
    if (options.body !== undefined) {
      init.body = JSON.stringify(options.body);
    }

    return this.withRetry(url, init);
  }

  private buildUrl(path: string, params?: Record<string, unknown>): string {
    const url = new URL(`${this.baseUrl}${path}`);
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        if (v !== undefined && v !== null) {
          url.searchParams.set(k, String(v));
        }
      }
    }
    return url.toString();
  }

  private async withRetry<T>(url: string, init: RequestInit, attempt = 0): Promise<T> {
    const res = await fetch(url, init);

    if (!res.ok) {
      if (RETRYABLE_STATUSES.has(res.status) && attempt < this.maxRetries) {
        const retryAfter = this.getRetryAfter(res, attempt);
        await sleep(retryAfter);
        return this.withRetry<T>(url, init, attempt + 1);
      }
      await this.throwApiError(res);
    }

    // 204 No Content
    if (res.status === 204) return undefined as T;
    return res.json() as Promise<T>;
  }

  private getRetryAfter(res: Response, attempt: number): number {
    const header = res.headers.get("Retry-After");
    if (header) {
      const seconds = parseInt(header, 10);
      if (!isNaN(seconds)) return seconds * 1000;
    }
    // Exponential backoff with jitter
    return this.retryDelay * 2 ** attempt + Math.random() * 100;
  }

  private async throwApiError(res: Response): Promise<never> {
    let body: Partial<ApiError> = {};
    try {
      body = await res.json();
    } catch {
      // ignore parse errors
    }
    throw new StellarInsightsError(
      res.status,
      body.error ?? "UNKNOWN_ERROR",
      body.message ?? res.statusText,
      body.request_id,
    );
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
