export class SDKError extends Error {
  constructor(
    message: string,
    public readonly details?: Record<string, unknown>,
    options?: { cause?: unknown },
  ) {
    super(message, options);
    this.name = "SDKError";
  }
}
