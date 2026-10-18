/**
 * Types and helpers for the backend's standard pagination envelope.
 *
 * Every list endpoint returns `PaginatedResponse<T>`. Page through results by
 * passing `pagination.next_cursor` back as the `cursor` query parameter —
 * cursors are opaque and must not be parsed or constructed client-side.
 *
 * See docs/API_PAGINATION.md for the full contract.
 */

export interface PageMeta {
  limit: number;
  /** Total items across all pages, or null when the endpoint can't count cheaply. */
  total: number | null;
  has_next: boolean;
  has_prev: boolean;
  next_cursor: string | null;
  prev_cursor: string | null;
  /** @deprecated Use `next_cursor` / `prev_cursor`. */
  offset: number | null;
  /** @deprecated Use `next_cursor`. */
  next_offset: number | null;
  /** @deprecated Use `prev_cursor`. */
  prev_offset: number | null;
}

export interface PageLinks {
  self: string;
  next: string | null;
  prev: string | null;
}

export interface PaginatedResponse<T> {
  data: T[];
  pagination: PageMeta;
  links?: PageLinks;
}

export interface PageRequest {
  limit?: number;
  cursor?: string | null;
}

/** Append `limit` / `cursor` to a URLSearchParams instance. */
export function appendPageParams(
  params: URLSearchParams,
  page?: PageRequest,
): URLSearchParams {
  if (page?.limit !== undefined) params.set("limit", String(page.limit));
  if (page?.cursor) params.set("cursor", page.cursor);
  return params;
}

/** `getNextPageParam` for `useInfiniteQuery` over a paginated endpoint. */
export function getNextCursor<T>(
  lastPage: PaginatedResponse<T>,
): string | undefined {
  return lastPage.pagination.next_cursor ?? undefined;
}
