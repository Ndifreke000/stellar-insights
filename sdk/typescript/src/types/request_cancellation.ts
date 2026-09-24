export interface CancellableRequestParams {
  /** Unique key identifying the request to cancel */
  requestKey: string;
  /** Optional timeout in ms before the request is auto-cancelled */
  timeoutMs?: number;
}

export interface CancellableRequestResult {
  success: true;
  requestKey: string;
  cancelled: boolean;
  data: {
    registeredAt: string;
    timeoutMs?: number;
  };
}
