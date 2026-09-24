import type { PayRaiderConfig } from "./types.js";
import type { CancellableRequestParams, CancellableRequestResult } from "./types/request_cancellation.js";
import { SDKError } from "./sdk_error.js";

export class RequestCancellation {
  private readonly config: PayRaiderConfig;
  private readonly controllers = new Map<string, AbortController>();

  constructor(config: PayRaiderConfig = {}) {
    this.config = config;
  }

  /**
   * Register a cancellable request and return its AbortSignal.
   * Calling execute() again with the same key cancels the previous one.
   */
  public async execute(params: CancellableRequestParams): Promise<CancellableRequestResult> {
    if (!params || typeof params.requestKey !== "string" || !params.requestKey.trim()) {
      throw new SDKError("requestKey is required", { params });
    }
    if (params.timeoutMs !== undefined && (typeof params.timeoutMs !== "number" || params.timeoutMs <= 0)) {
      throw new SDKError("timeoutMs must be a positive number", { params });
    }

    try {
      const key = params.requestKey.trim();

      // Cancel any in-flight request with the same key
      const existing = this.controllers.get(key);
      const cancelled = existing !== undefined;
      existing?.abort();

      const controller = new AbortController();
      this.controllers.set(key, controller);

      if (params.timeoutMs !== undefined) {
        setTimeout(() => this.cancel(key), params.timeoutMs);
      }

      return {
        success: true,
        requestKey: key,
        cancelled,
        data: {
          registeredAt: new Date().toISOString(),
          timeoutMs: params.timeoutMs,
        },
      };
    } catch (error) {
      if (error instanceof SDKError) throw error;
      throw new SDKError("Failed to register cancellable request", { params }, { cause: error });
    }
  }

  /** Cancel a registered request by key. Returns true if it was found and aborted. */
  public cancel(requestKey: string): boolean {
    const controller = this.controllers.get(requestKey);
    if (!controller) return false;
    controller.abort();
    this.controllers.delete(requestKey);
    return true;
  }

  /** Returns the AbortSignal for a registered request key, or undefined if not found. */
  public getSignal(requestKey: string): AbortSignal | undefined {
    return this.controllers.get(requestKey)?.signal;
  }

  /** Cancel all in-flight requests. */
  public cancelAll(): void {
    for (const controller of this.controllers.values()) {
      controller.abort();
    }
    this.controllers.clear();
  }
}
