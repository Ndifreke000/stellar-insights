export interface RequestDeduplicationParams<T = unknown> {
  /** Unique key identifying the request (e.g. URL + serialized params) */
  requestKey: string;
  /** Async function that performs the actual request */
  fetcher: () => Promise<T>;
  /** When true, bypass deduplication and always execute fetcher */
  skipDeduplication?: boolean;
}

export interface RequestDeduplicationResult<T = unknown> {
  success: true;
  requestKey: string;
  /** True when this call reused an in-flight request for the same key */
  deduplicated: boolean;
  data: T;
}

export interface RequestDeduplicationMetrics {
  totalRequests: number;
  deduplicationHits: number;
  inFlightCount: number;
}
