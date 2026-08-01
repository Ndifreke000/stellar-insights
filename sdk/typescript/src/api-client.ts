import type { StellarInsightsConfig, ApiError, PaginatedResponse } from "./types.js";
import { HttpClient, StellarInsightsError } from "./http.js";

/**
 * Request interceptor type
 */
export type RequestInterceptor = (
  path: string,
  options: RequestOptions
) => RequestOptions;

/**
 * Response interceptor type
 */
export type ResponseInterceptor = <T>(response: T) => T;

/**
 * Error interceptor type
 */
export type ErrorInterceptor = (error: Error) => Error;

/**
 * Request options
 */
export interface RequestOptions {
  body?: unknown;
  params?: Record<string, unknown>;
  headers?: Record<string, string>;
}

/**
 * API endpoint definition
 */
export interface ApiEndpoint<TRequest = void, TResponse = void> {
  method: "GET" | "POST" | "PUT" | "DELETE" | "PATCH";
  path: string;
  description?: string;
  requiresAuth?: boolean;
}

/**
 * API response wrapper
 */
export interface ApiResponse<T> {
  data: T;
  status: number;
  headers: Record<string, string>;
  timestamp: Date;
  requestId?: string;
}

/**
 * API client error with enhanced context
 */
export class ApiClientError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly code: string,
    public readonly requestId?: string,
    public readonly originalError?: Error,
    public readonly retryable: boolean = false
  ) {
    super(message);
    this.name = "ApiClientError";
  }

  /**
   * Check if error is retryable
   */
  isRetryable(): boolean {
    return this.retryable || [429, 500, 502, 503, 504].includes(this.status);
  }

  /**
   * Check if error is a network error
   */
  isNetworkError(): boolean {
    return this.originalError instanceof TypeError || !navigator.onLine;
  }

  /**
   * Get user-friendly error message
   */
  getUserMessage(): string {
    if (this.status >= 500) {
      return "Server error. Please try again later.";
    }
    if (this.status === 429) {
      return "Too many requests. Please wait before trying again.";
    }
    if (this.status === 401) {
      return "Authentication failed. Please check your credentials.";
    }
    if (this.status === 403) {
      return "You don't have permission to access this resource.";
    }
    if (this.status === 404) {
      return "Resource not found.";
    }
    if (this.isNetworkError()) {
      return "Network connection error. Please check your internet connection.";
    }
    return this.message || "An error occurred. Please try again.";
  }
}

/**
 * Core API Client with enhanced functionality for web and mobile
 */
export class ApiClient {
  private httpClient: HttpClient;
  private requestInterceptors: RequestInterceptor[] = [];
  private responseInterceptors: ResponseInterceptor[] = [];
  private errorInterceptors: ErrorInterceptor[] = [];
  private requestCache: Map<string, { data: unknown; timestamp: number }> =
    new Map();
  private cacheTimeout: number;
  private requestTimeout: number;
  private maxRetries: number;
  private retryDelay: number;

  constructor(config: StellarInsightsConfig = {}) {
    this.httpClient = new HttpClient(config);
    this.cacheTimeout = (config as any).cacheTimeout ?? 60;
    this.requestTimeout = config.timeout ?? 30000;
    this.maxRetries = config.maxRetries ?? 3;
    this.retryDelay = config.retryDelay ?? 500;
  }

  /**
   * Add a request interceptor
   */
  addRequestInterceptor(interceptor: RequestInterceptor): void {
    this.requestInterceptors.push(interceptor);
  }

  /**
   * Add a response interceptor
   */
  addResponseInterceptor(interceptor: ResponseInterceptor): void {
    this.responseInterceptors.push(interceptor);
  }

  /**
   * Add an error interceptor
   */
  addErrorInterceptor(interceptor: ErrorInterceptor): void {
    this.errorInterceptors.push(interceptor);
  }

  /**
   * Execute request interceptors
   */
  private executeRequestInterceptors(
    path: string,
    options: RequestOptions
  ): RequestOptions {
    let result = options;
    for (const interceptor of this.requestInterceptors) {
      result = interceptor(path, result);
    }
    return result;
  }

  /**
   * Execute response interceptors
   */
  private executeResponseInterceptors<T>(response: T): T {
    let result: unknown = response;
    for (const interceptor of this.responseInterceptors) {
      result = interceptor(result);
    }
    return result as T;
  }

  /**
   * Execute error interceptors
   */
  private executeErrorInterceptors(error: Error): Error {
    let result = error;
    for (const interceptor of this.errorInterceptors) {
      result = interceptor(result);
    }
    return result;
  }

  /**
   * Get from cache if available
   */
  private getFromCache(key: string): unknown | null {
    if (this.cacheTimeout <= 0) return null;

    const cached = this.requestCache.get(key);
    if (!cached) return null;

    const age = Date.now() - cached.timestamp;
    if (age > this.cacheTimeout * 1000) {
      this.requestCache.delete(key);
      return null;
    }

    return cached.data;
  }

  /**
   * Set cache
   */
  private setCache(key: string, data: unknown): void {
    if (this.cacheTimeout <= 0) return;
    this.requestCache.set(key, { data, timestamp: Date.now() });
  }

  /**
   * Generate cache key
   */
  private getCacheKey(
    method: string,
    path: string,
    params?: Record<string, unknown>
  ): string {
    const paramStr = params ? JSON.stringify(params) : "";
    return `${method}:${path}:${paramStr}`;
  }

  /**
   * Make an API request with automatic retry and error handling
   */
  async request<T>(
    method: string,
    path: string,
    options: RequestOptions = {},
    skipCache: boolean = false
  ): Promise<T> {
    const cacheKey = this.getCacheKey(method, path, options.params);

    // Check cache for GET requests
    if (method === "GET" && !skipCache) {
      const cached = this.getFromCache(cacheKey);
      if (cached !== null) {
        return this.executeResponseInterceptors(cached as T);
      }
    }

    // Apply request interceptors
    const interceptedOptions = this.executeRequestInterceptors(path, options);

    try {
      const response = await this.requestWithRetry<T>(
        method,
        path,
        interceptedOptions
      );

      // Cache successful GET responses
      if (method === "GET") {
        this.setCache(cacheKey, response);
      }

      return this.executeResponseInterceptors(response);
    } catch (error) {
      const apiError = this.normalizeError(error);
      const handledError = this.executeErrorInterceptors(apiError);
      throw handledError;
    }
  }

  /**
   * Make a GET request
   */
  async get<T>(
    path: string,
    params?: Record<string, unknown>,
    skipCache?: boolean
  ): Promise<T> {
    return this.request<T>("GET", path, { params }, skipCache);
  }

  /**
   * Make a POST request
   */
  async post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("POST", path, { body });
  }

  /**
   * Make a PUT request
   */
  async put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("PUT", path, { body });
  }

  /**
   * Make a DELETE request
   */
  async delete<T>(path: string): Promise<T> {
    return this.request<T>("DELETE", path);
  }

  /**
   * Make a PATCH request
   */
  async patch<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>("PATCH", path, { body });
  }

  /**
   * Execute request with automatic retry
   */
  private async requestWithRetry<T>(
    method: string,
    path: string,
    options: RequestOptions,
    attempt: number = 0
  ): Promise<T> {
    try {
      return await this.httpClient.request<T>(method, path, options);
    } catch (error) {
      const apiError = this.normalizeError(error);

      if (
        apiError.isRetryable() &&
        attempt < this.maxRetries
      ) {
        const delay =
          this.retryDelay * Math.pow(2, attempt) + Math.random() * 100;
        await new Promise((resolve) => setTimeout(resolve, delay));
        return this.requestWithRetry<T>(
          method,
          path,
          options,
          attempt + 1
        );
      }

      throw apiError;
    }
  }

  /**
   * Normalize errors to ApiClientError
   */
  private normalizeError(error: unknown): ApiClientError {
    if (error instanceof ApiClientError) {
      return error;
    }

    if (error instanceof StellarInsightsError) {
      return new ApiClientError(
        error.message,
        error.status,
        error.code,
        error.requestId,
        error,
        [429, 500, 502, 503, 504].includes(error.status)
      );
    }

    if (error instanceof TypeError) {
      return new ApiClientError(
        "Network error occurred",
        0,
        "NETWORK_ERROR",
        undefined,
        error as Error,
        true
      );
    }

    if (error instanceof Error) {
      return new ApiClientError(
        error.message,
        0,
        "UNKNOWN_ERROR",
        undefined,
        error,
        false
      );
    }

    return new ApiClientError(
      "An unknown error occurred",
      0,
      "UNKNOWN_ERROR",
      undefined,
      undefined,
      false
    );
  }

  /**
   * Clear the request cache
   */
  clearCache(): void {
    this.requestCache.clear();
  }

  /**
   * Get cache statistics
   */
  getCacheStats() {
    return {
      size: this.requestCache.size,
      timeout: this.cacheTimeout,
    };
  }

  /**
   * Set the token for authentication
   */
  setToken(token: string): void {
    this.httpClient.setToken(token);
  }
}

/**
 * Batch request handler for efficient data loading on mobile
 */
export class BatchApiClient extends ApiClient {
  private batchQueue: Array<{
    method: string;
    path: string;
    options: RequestOptions;
    resolve: (value: unknown) => void;
    reject: (reason?: Error) => void;
  }> = [];
  private batchTimeout: NodeJS.Timeout | null = null;
  private batchSize: number;
  private batchDelayMs: number;

  constructor(config: StellarInsightsConfig = {}, batchSize: number = 10, batchDelayMs: number = 100) {
    super(config);
    this.batchSize = batchSize;
    this.batchDelayMs = batchDelayMs;
  }

  /**
   * Queue a request for batch processing
   */
  async batchRequest<T>(
    method: string,
    path: string,
    options?: RequestOptions
  ): Promise<T> {
    return new Promise((resolve, reject) => {
      // The queue holds requests for many different T's, so it's typed with
      // an erased `unknown` resolver - each caller's own Promise<T> executor
      // still resolves with the right value, this cast just matches the
      // heterogeneous queue's storage type.
      this.batchQueue.push({
        method,
        path,
        options: options || {},
        resolve: resolve as (value: unknown) => void,
        reject,
      });

      if (this.batchQueue.length >= this.batchSize) {
        this.processBatch();
      } else if (!this.batchTimeout) {
        this.batchTimeout = setTimeout(() => this.processBatch(), this.batchDelayMs);
      }
    });
  }

  /**
   * Process queued batch requests
   */
  private async processBatch(): Promise<void> {
    if (this.batchTimeout) {
      clearTimeout(this.batchTimeout);
      this.batchTimeout = null;
    }

    const queue = this.batchQueue.splice(0, this.batchSize);
    if (queue.length === 0) return;

    for (const item of queue) {
      try {
        const response = await super.request(
          item.method,
          item.path,
          item.options
        );
        item.resolve(response);
      } catch (error) {
        item.reject(error as Error);
      }
    }
  }

  /**
   * Flush any pending batch requests
   */
  async flush(): Promise<void> {
    if (this.batchTimeout) {
      clearTimeout(this.batchTimeout);
      this.batchTimeout = null;
    }
    if (this.batchQueue.length > 0) {
      await this.processBatch();
    }
  }
}

export default ApiClient;
