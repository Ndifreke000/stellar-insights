/**
 * SEP-24 (Hosted Deposit and Withdrawal) client service.
 * All requests go through the backend proxy to avoid CORS and centralize auth.
 */

const API_BASE =
  typeof process !== "undefined"
    ? process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080"
    : "";

export class Sep24Error extends Error {
  constructor(
    message: string,
    public status?: number,
    public data?: unknown
  ) {
    super(message);
    this.name = "Sep24Error";
  }
}

async function fetchSep24<T>(
  endpoint: string,
  options: RequestInit = {}
): Promise<T> {
  const url = endpoint.startsWith("http") ? endpoint : `${API_BASE}${endpoint}`;
  const res = await fetch(url, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      ...(options.headers as Record<string, string>),
    },
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    const msg =
      (data as { message?: string })?.message ||
      (data as { error?: string })?.error ||
      res.statusText;
    throw new Sep24Error(msg, res.status, data);
  }
  return data as T;
}

// --- Types (align with SEP-24 and backend proxy) ---

export interface Sep24AnchorInfo {
  name: string;
  transfer_server: string;
  home_domain?: string;
}

export interface Sep24AnchorsResponse {
  anchors: Sep24AnchorInfo[];
}

/** Anchor /info response (deposit/withdraw capabilities per asset) */
export interface Sep24InfoResponse {
  deposit?: Record<string, Sep24AssetInfo>;
  withdraw?: Record<string, Sep24AssetInfo>;
  fee?: { enabled: boolean };
  [key: string]: unknown;
}

export interface Sep24AssetInfo {
  enabled: boolean;
  min_amount?: number;
  max_amount?: number;
  fee_fixed?: number;
  fee_percent?: number;
  authentication_required?: boolean;
  methods?: string[];
  [key: string]: unknown;
}

/** Response from deposit/withdraw interactive (url to open in popup) */
export interface Sep24InteractiveResponse {
  type?: string;
  url?: string;
  id?: string;
  transaction?: Sep24Transaction;
  [key: string]: unknown;
}

export interface Sep24Transaction {
  id: string;
  kind: "deposit" | "withdraw";
  status: string;
  amount_in?: string;
  amount_out?: string;
  amount_fee?: string;
  from?: string;
  to?: string;
  started_at?: string;
  completed_at?: string;
  asset_code?: string;
  [key: string]: unknown;
}

export interface Sep24TransactionsResponse {
  transactions: Sep24Transaction[];
}

// --- API ---

/**
 * List SEP-24-enabled anchors (from backend config).
 */
export async function getSep24Anchors(): Promise<Sep24AnchorsResponse> {
  return fetchSep24<Sep24AnchorsResponse>("/api/sep24/anchors");
}

/**
 * Get anchor capabilities (deposit/withdraw assets and limits).
 */
export async function getSep24Info(
  transferServer: string
): Promise<Sep24InfoResponse> {
  const params = new URLSearchParams({ transfer_server: transferServer });
  return fetchSep24<Sep24InfoResponse>(`/api/sep24/info?${params}`);
}

export interface DepositInteractiveParams {
  transfer_server: string;
  asset_code?: string;
  account?: string;
  memo?: string;
  memo_type?: string;
  email?: string;
  amount?: string;
  lang?: string;
  jwt?: string;
}

/**
 * Start interactive deposit flow. Returns URL to open in popup/iframe.
 */
export async function startDepositInteractive(
  params: DepositInteractiveParams
): Promise<Sep24InteractiveResponse> {
  return fetchSep24<Sep24InteractiveResponse>("/api/sep24/deposit/interactive", {
    method: "POST",
    body: JSON.stringify(params),
  });
}

export interface WithdrawInteractiveParams {
  transfer_server: string;
  asset_code?: string;
  account?: string;
  memo?: string;
  memo_type?: string;
  dest?: string;
  dest_extra?: string;
  amount?: string;
  lang?: string;
  jwt?: string;
}

/**
 * Start interactive withdrawal flow. Returns URL to open in popup/iframe.
 */
export async function startWithdrawInteractive(
  params: WithdrawInteractiveParams
): Promise<Sep24InteractiveResponse> {
  return fetchSep24<Sep24InteractiveResponse>(
    "/api/sep24/withdraw/interactive",
    {
      method: "POST",
      body: JSON.stringify(params),
    }
  );
}

export interface GetTransactionsParams {
  transfer_server: string;
  jwt?: string;
  asset_code?: string;
  kind?: "deposit" | "withdraw";
  limit?: number;
  cursor?: string;
}

/**
 * Get transaction history from anchor.
 */
export async function getSep24Transactions(
  params: GetTransactionsParams
): Promise<Sep24TransactionsResponse> {
  const search = new URLSearchParams();
  search.set("transfer_server", params.transfer_server);
  if (params.jwt) search.set("jwt", params.jwt);
  if (params.asset_code) search.set("asset_code", params.asset_code);
  if (params.kind) search.set("kind", params.kind);
  if (params.limit != null) search.set("limit", String(params.limit));
  if (params.cursor) search.set("cursor", params.cursor);
  return fetchSep24<Sep24TransactionsResponse>(
    `/api/sep24/transactions?${search}`
  );
}

/**
 * Get a single transaction by id.
 */
export async function getSep24Transaction(
  transferServer: string,
  id: string,
  jwt?: string
): Promise<{ transaction: Sep24Transaction }> {
  const params = new URLSearchParams({
    transfer_server: transferServer,
    id,
  });
  if (jwt) params.set("jwt", jwt);
  return fetchSep24<{ transaction: Sep24Transaction }>(
    `/api/sep24/transaction?${params}`
  );
}
