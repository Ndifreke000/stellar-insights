import type { HttpClient } from "./http.js";
import type { PaginationParams } from "./types.js";

export interface PaginatedResult<T> {
  data: T[];
  cursor?: string;
}

/**
 * Helper to paginate through API results with cursor tracking.
 * Detects wrap-around (cursor seen twice) and breaks to prevent infinite loops.
 * Falls back to data length detection for normal pagination termination.
 *
 * @param httpClient - HTTP client to use for requests
 * @param endpoint - API endpoint path
 * @param pageSize - Number of items per page
 * @param options - Optional pagination parameters and request options
 * @yields Paginated results until end-of-results or cursor wrap-around detected
 */
export async function* paginate<T>(
  httpClient: HttpClient,
  endpoint: string,
  pageSize: number = 100,
  options?: { params?: PaginationParams; headers?: Record<string, string> },
): AsyncGenerator<PaginatedResult<T>> {
  const seenCursors = new Set<string>();
  let cursor: string | undefined;
  let page = 0;

  while (true) {
    const params = { ...options?.params, limit: pageSize };
    if (cursor) {
      (params as Record<string, unknown>).cursor = cursor;
    }

    const result = await httpClient.request<PaginatedResult<T>>("GET", endpoint, {
      params: params as Record<string, unknown>,
      headers: options?.headers,
    });

    yield result;

    // Check if cursor was seen before (wrap-around detection)
    if (cursor && seenCursors.has(cursor)) {
      break;
    }

    if (cursor) {
      seenCursors.add(cursor);
    }

    // End of results: data length < page size or no cursor in response
    if (result.data.length < pageSize || !result.cursor) {
      break;
    }

    cursor = result.cursor;
    page++;
  }
}

/**
 * Collect all paginated results into a single array.
 *
 * @param httpClient - HTTP client to use for requests
 * @param endpoint - API endpoint path
 * @param pageSize - Number of items per page
 * @param options - Optional pagination parameters and request options
 * @returns Array of all items from all pages
 */
export async function paginateAll<T>(
  httpClient: HttpClient,
  endpoint: string,
  pageSize: number = 100,
  options?: { params?: PaginationParams; headers?: Record<string, string> },
): Promise<T[]> {
  const all: T[] = [];
  for await (const result of paginate(httpClient, endpoint, pageSize, options)) {
    all.push(...result.data);
  }
  return all;
}
