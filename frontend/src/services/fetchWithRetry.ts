import { config } from '@/config';

const API_BASE = config.apiUrl;

const MAX_RETRY_DELAY_MS = 5000;

export async function fetchWithRetry<T>(
  endpoint: string,
  options: RequestInit = {},
  ErrorClass: new (message: string, status?: number, data?: unknown) => Error & { status?: number; data?: unknown },
): Promise<T> {
  const url = endpoint.startsWith("http") ? endpoint : `${API_BASE}${endpoint}`;
  const init: RequestInit = {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(options.headers as Record<string, string>),
    },
  };

  let res = await fetch(url, init);

  if (res.status === 503) {
    const retryAfterHeader = res.headers.get("Retry-After");
    const delayMs = retryAfterHeader
      ? Math.min(parseInt(retryAfterHeader, 10) * 1000 || 1000, MAX_RETRY_DELAY_MS)
      : 1000;
    await new Promise((resolve) => setTimeout(resolve, delayMs));
    res = await fetch(url, init);
  }

  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    const msg =
      (data as { message?: string })?.message ||
      (data as { error?: string })?.error ||
      res.statusText;
    throw new ErrorClass(msg, res.status, data);
  }
  return data as T;
}
