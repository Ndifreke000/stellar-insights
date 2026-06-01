/**
 * SEP-31 (Cross-Border Payments) client service.
 * All requests go through the backend proxy.
 */

const API_BASE =
  typeof process !== "undefined"
    ? process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080"
    : "";

export class Sep31Error extends Error {
  constructor(
    message: string,
    public status?: number,
    public data?: unknown
  ) {
    super(message);
    this.name = "Sep31Error";
  }
}

async function fetchSep31<T>(
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
    throw new Sep31Error(msg, res.status, data);
  }
  return data as T;
}

// --- Types ---

export interface Sep31AnchorInfo {
  name: string;
  transfer_server: string;
  home_domain?: string;
}

export interface Sep31AnchorsResponse {
  anchors: Sep31AnchorInfo[];
}

/** Anchor /info response (receive methods, currencies, quotes_supported) */
export interface Sep31InfoResponse {
  receive?: Record<string, Sep31ReceiveMethod>;
  quotes_supported?: boolean;
  quotes_required?: boolean;
  [key: string]: unknown;
}

export interface Sep31ReceiveMethod {
  enabled: boolean;
  min_amount?: number;
  max_amount?: number;
  currencies?: string[];
  [key: string]: unknown;
}

/** Quote request (SEP-38 style: sell_asset, buy_asset, amount) */
export interface Sep31QuoteParams {
  transfer_server: string;
  jwt?: string;
  amount?: string;
  sell_asset?: string;
  buy_asset?: string;
  sell_delivery_method?: string;
  buy_delivery_method?: string;
  [key: string]: unknown;
}

/** Quote response */
export interface Sep31QuoteResponse {
  id?: string;
  total_price?: string;
  price?: string;
  sell_asset?: string;
  buy_asset?: string;
  expires_at?: string;
  [key: string]: unknown;
}

/** Create payment (transaction) request */
export interface Sep31CreatePaymentParams {
  transfer_server: string;
  jwt?: string;
  amount: string;
  source_asset?: string;
  destination_asset?: string;
  receiver_id?: string;
  sender_id?: string;
  quote_id?: string;
  [key: string]: unknown;
}

/** Transaction (payment) from anchor */
export interface Sep31Transaction {
  id: string;
  status: string;
  amount?: string;
  amount_out?: string;
  source_asset?: string;
  destination_asset?: string;
  receiver_id?: string;
  sender_id?: string;
  created_at?: string;
  updated_at?: string;
  completed_at?: string;
  [key: string]: unknown;
}

export interface Sep31TransactionsResponse {
  transactions: Sep31Transaction[];
}

// --- API ---

export async function getSep31Anchors(): Promise<Sep31AnchorsResponse> {
  return fetchSep31<Sep31AnchorsResponse>("/api/sep31/anchors");
}

export async function getSep31Info(
  transferServer: string
): Promise<Sep31InfoResponse> {
  const params = new URLSearchParams({ transfer_server: transferServer });
  return fetchSep31<Sep31InfoResponse>(`/api/sep31/info?${params}`);
}

export async function getSep31Quote(
  params: Sep31QuoteParams
): Promise<Sep31QuoteResponse> {
  const { transfer_server, jwt, ...payload } = params;
  return fetchSep31<Sep31QuoteResponse>("/api/sep31/quote", {
    method: "POST",
    body: JSON.stringify({
      transfer_server,
      jwt,
      payload: payload as Record<string, unknown>,
    }),
  });
}

export async function createSep31Payment(
  params: Sep31CreatePaymentParams
): Promise<{ transaction: Sep31Transaction; id?: string }> {
  const { transfer_server, jwt, ...payload } = params;
  return fetchSep31<{ transaction: Sep31Transaction; id?: string }>(
    "/api/sep31/transactions",
    {
      method: "POST",
      body: JSON.stringify({
        transfer_server,
        jwt,
        payload: payload as Record<string, unknown>,
      }),
    }
  );
}

export interface GetTransactionsParams {
  transfer_server: string;
  jwt?: string;
  status?: string;
  limit?: number;
  cursor?: string;
}

export async function getSep31Transactions(
  params: GetTransactionsParams
): Promise<Sep31TransactionsResponse> {
  const search = new URLSearchParams();
  search.set("transfer_server", params.transfer_server);
  if (params.jwt) search.set("jwt", params.jwt);
  if (params.status) search.set("status", params.status);
  if (params.limit != null) search.set("limit", String(params.limit));
  if (params.cursor) search.set("cursor", params.cursor);
  return fetchSep31<Sep31TransactionsResponse>(
    `/api/sep31/transactions?${search}`
  );
}

export async function getSep31Transaction(
  transferServer: string,
  id: string,
  jwt?: string
): Promise<{ transaction: Sep31Transaction }> {
  const params = new URLSearchParams({ transfer_server: transferServer });
  if (jwt) params.set("jwt", jwt);
  return fetchSep31<{ transaction: Sep31Transaction }>(
    `/api/sep31/transactions/${encodeURIComponent(id)}?${params}`
  );
}

export async function getSep31Customer(
  transferServer: string,
  customerId: string,
  jwt?: string
): Promise<unknown> {
  const params = new URLSearchParams({
    transfer_server: transferServer,
    id: customerId,
  });
  if (jwt) params.set("jwt", jwt);
  return fetchSep31(`/api/sep31/customer?${params}`);
}
