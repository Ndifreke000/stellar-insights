import type { PayRaiderConfig, ApiError } from "./types.js";
import {
  DEFAULT_BASE_DELAY_MS,
  DEFAULT_MAX_DELAY_MS,
  DEFAULT_MAX_RETRIES,
  RETRYABLE_STATUSES,
  fullJitterDelay,
  isIdempotentMethod,
  parseRetryAfter,
} from "./retry_policy.js";

const DEFAULT_BASE_URL = "https://api.payraider.io";
const DEFAULT_TIMEOUT = 30_000;

export class PayRaiderError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
    public readonly requestId?: string,
    /** Parsed `Retry-After` header in ms, when the server sent one */
    public readonly retryAfterMs?: number,
  ) {
    super(message);
    this.name = "PayRaiderError";
  }
}

export class HttpClient {
  private readonly baseUrl: string;
  private readonly maxRetries: number;
  private readonly retryDelay: number;
  private readonly maxRetryDelay: number;
  private readonly timeout: number;
  private token: string | undefined;

  constructor(config: PayRaiderConfig) {
    this.baseUrl = (config.baseUrl ?? DEFAULT_BASE_URL).replace(/\/$/, "");
    this.maxRetries = config.maxRetries ?? DEFAULT_MAX_RETRIES;
    this.retryDelay = config.retryDelay ?? DEFAULT_BASE_DELAY_MS;
    this.maxRetryDelay = config.maxRetryDelay ?? DEFAULT_MAX_DELAY_MS;
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
    options: {
      body?: unknown;
      params?: Record<string, unknown>;
      headers?: Record<string, string>;
      /** Allow retries for a non-idempotent method (e.g. a read-only POST) */
      idempotent?: boolean;
    } = {},
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

    const init: RequestInit = { method, headers };
    if (options.body !== undefined) {
      init.body = JSON.stringify(options.body);
    }

    const retryable = options.idempotent ?? isIdempotentMethod(method);
    return this.withRetry(url, init, retryable ? this.maxRetries : 0);
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

  private async withRetry<T>(url: string, init: RequestInit, maxRetries: number): Promise<T> {
    for (let attempt = 0; ; attempt++) {
      let res: Response;
      try {
        // Fresh timeout per attempt so retries are not starved by earlier ones.
        res = await fetch(url, { ...init, signal: AbortSignal.timeout(this.timeout) });
      } catch (error) {
        if (attempt < maxRetries && isNetworkError(error)) {
          await sleep(fullJitterDelay(attempt, this.retryDelay, this.maxRetryDelay));
          continue;
        }
        throw error;
      }

      if (res.ok) {
        // 204 No Content
        if (res.status === 204) return undefined as T;
        return res.json() as Promise<T>;
      }

      const retryAfterMs = parseRetryAfter(res.headers.get("Retry-After"));
      if (
        RETRYABLE_STATUSES.has(res.status) &&
        attempt < maxRetries &&
        (retryAfterMs === undefined || retryAfterMs <= this.maxRetryDelay)
      ) {
        await sleep(retryAfterMs ?? fullJitterDelay(attempt, this.retryDelay, this.maxRetryDelay));
        continue;
      }
      return this.throwApiError(res, retryAfterMs);
    }
  }

  private async throwApiError(res: Response, retryAfterMs?: number): Promise<never> {
    let raw: unknown;
    try {
      raw = await res.json();
    } catch {
      // ignore parse errors
    }
    const body = parseErrorBody(raw);
    throw new PayRaiderError(
      res.status,
      body.error ?? "UNKNOWN_ERROR",
      body.message ?? res.statusText,
      body.request_id,
      retryAfterMs,
    );
  }
}

/**
 * The API sends `{"error": {"code", "message", "request_id"}}`; older handlers
 * send the flat `{"error": "CODE", "message", "request_id"}`. Accept both.
 */
function parseErrorBody(raw: unknown): Partial<Pick<ApiError, "error" | "message" | "request_id">> {
  if (!raw || typeof raw !== "object") return {};
  const obj = raw as Record<string, unknown>;
  const source =
    obj.error && typeof obj.error === "object" ? (obj.error as Record<string, unknown>) : obj;
  const str = (v: unknown) => (typeof v === "string" ? v : undefined);
  return {
    error: str(source === obj ? obj.error : source.code),
    message: str(source.message),
    request_id: str(source.request_id),
  };
}

/** fetch rejects with TypeError on connection failures and DOMException on timeout. */
function isNetworkError(error: unknown): boolean {
  return error instanceof TypeError || (error instanceof Error && error.name === "TimeoutError");
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
