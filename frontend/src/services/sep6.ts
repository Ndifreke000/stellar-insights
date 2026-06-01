/**
 * SEP-6 (Deposit and Withdrawal API) client service.
 * Programmatic deposit/withdraw flows; requests go through backend proxy when configured.
 */

const API_BASE =
  typeof process !== "undefined"
    ? process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080"
    : "";

export class Sep6Error extends Error {
  constructor(
    message: string,
    public status?: number,
    public data?: unknown
  ) {
    super(message);
    this.name = "Sep6Error";
  }
}

async function fetchSep6<T>(
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
    throw new Sep6Error(msg, res.status, data);
  }
  return data as T;
}

// --- Types (align with SEP-6) ---

export interface Sep6AnchorInfo {
  name: string;
  transfer_server: string;
  home_domain?: string;
}

export interface Sep6AnchorsResponse {
  anchors: Sep6AnchorInfo[];
}

/** Field descriptor from /info */
export interface Sep6Field {
  name: string;
  description?: string;
  optional?: boolean;
  choices?: string[];
  [key: string]: unknown;
}

/** Deposit/withdraw method from /info */
export interface Sep6Method {
  type: string;
  name?: string;
  fields?: Record<string, Sep6Field>;
  [key: string]: unknown;
}

export interface Sep6InfoResponse {
  deposit?: Record<string, Sep6Method>;
  withdraw?: Record<string, Sep6Method>;
  fee?: { enabled: boolean };
  [key: string]: unknown;
}

/** Deposit response: how to deposit (type, url, id, etc.) */
export interface Sep6DepositResponse {
  type?: string;
  url?: string;
  id?: string;
  how?: string;
  eta?: number;
  min_amount?: number;
  max_amount?: number;
  fee_fixed?: number;
  fee_percent?: number;
  extra_info?: Record<string, string>;
  [key: string]: unknown;
}

/** Withdraw response */
export interface Sep6WithdrawResponse {
  type?: string;
  url?: string;
  id?: string;
  how?: string;
  eta?: number;
  min_amount?: number;
  max_amount?: number;
  fee_fixed?: number;
  fee_percent?: number;
  extra_info?: Record<string, string>;
  [key: string]: unknown;
}

export interface Sep6Transaction {
  id: string;
  kind: "deposit" | "withdraw";
  status: string;
  status_eta?: number;
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

export interface Sep6TransactionsResponse {
  transactions: Sep6Transaction[];
}

// --- API (backend proxy: /api/sep6/*?transfer_server=... or body) ---

/**
 * List SEP-6-enabled anchors (from backend config).
 */
export async function getSep6Anchors(): Promise<Sep6AnchorsResponse> {
  return fetchSep6<Sep6AnchorsResponse>("/api/sep6/anchors");
}

/**
 * Get anchor capabilities (deposit/withdraw methods and fields).
 */
export async function getSep6Info(
  transferServer: string
): Promise<Sep6InfoResponse> {
  const params = new URLSearchParams({ transfer_server: transferServer });
  return fetchSep6<Sep6InfoResponse>(`/api/sep6/info?${params}`);
}

export interface Sep6DepositParams {
  transfer_server: string;
  asset_code: string;
  account: string;
  memo?: string;
  memo_type?: string;
  email?: string;
  amount?: string;
  type?: string;
  wallet_name?: string;
  wallet_url?: string;
  lang?: string;
  jwt?: string;
  [key: string]: string | undefined;
}

/**
 * Request deposit instructions (SEP-6 GET /deposit).
 */
export async function getSep6Deposit(
  params: Sep6DepositParams
): Promise<Sep6DepositResponse> {
  const search = new URLSearchParams();
  search.set("transfer_server", params.transfer_server);
  search.set("asset_code", params.asset_code);
  search.set("account", params.account);
  if (params.memo) search.set("memo", params.memo);
  if (params.memo_type) search.set("memo_type", params.memo_type);
  if (params.email) search.set("email", params.email);
  if (params.amount) search.set("amount", params.amount);
  if (params.type) search.set("type", params.type);
  if (params.lang) search.set("lang", params.lang);
  if (params.jwt) search.set("jwt", params.jwt);
  return fetchSep6<Sep6DepositResponse>(`/api/sep6/deposit?${search}`);
}

export interface Sep6WithdrawParams {
  transfer_server: string;
  asset_code: string;
  account: string;
  type: string;
  dest?: string;
  dest_extra?: string;
  amount?: string;
  memo?: string;
  memo_type?: string;
  wallet_name?: string;
  wallet_url?: string;
  lang?: string;
  jwt?: string;
  [key: string]: string | undefined;
}

/**
 * Request withdrawal instructions (SEP-6 GET /withdraw).
 */
export async function getSep6Withdraw(
  params: Sep6WithdrawParams
): Promise<Sep6WithdrawResponse> {
  const search = new URLSearchParams();
  search.set("transfer_server", params.transfer_server);
  search.set("asset_code", params.asset_code);
  search.set("account", params.account);
  search.set("type", params.type);
  if (params.dest) search.set("dest", params.dest);
  if (params.dest_extra) search.set("dest_extra", params.dest_extra);
  if (params.amount) search.set("amount", params.amount);
  if (params.memo) search.set("memo", params.memo);
  if (params.memo_type) search.set("memo_type", params.memo_type);
  if (params.lang) search.set("lang", params.lang);
  if (params.jwt) search.set("jwt", params.jwt);
  return fetchSep6<Sep6WithdrawResponse>(`/api/sep6/withdraw?${search}`);
}

export interface GetSep6TransactionsParams {
  transfer_server: string;
  jwt?: string;
  asset_code?: string;
  kind?: "deposit" | "withdraw";
  limit?: number;
  cursor?: string;
}

/**
 * List transactions from anchor.
 */
export async function getSep6Transactions(
  params: GetSep6TransactionsParams
): Promise<Sep6TransactionsResponse> {
  const search = new URLSearchParams();
  search.set("transfer_server", params.transfer_server);
  if (params.jwt) search.set("jwt", params.jwt);
  if (params.asset_code) search.set("asset_code", params.asset_code);
  if (params.kind) search.set("kind", params.kind);
  if (params.limit != null) search.set("limit", String(params.limit));
  if (params.cursor) search.set("cursor", params.cursor);
  return fetchSep6<Sep6TransactionsResponse>(
    `/api/sep6/transactions?${search}`
  );
}

/**
 * Get a single transaction by id.
 */
export async function getSep6Transaction(
  transferServer: string,
  id: string,
  jwt?: string
): Promise<{ transaction: Sep6Transaction }> {
  const params = new URLSearchParams({
    transfer_server: transferServer,
    id,
  });
  if (jwt) params.set("jwt", jwt);
  return fetchSep6<{ transaction: Sep6Transaction }>(
    `/api/sep6/transaction?${params}`
  );
}
