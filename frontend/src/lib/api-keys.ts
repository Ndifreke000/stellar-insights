import { config } from '@/config';
const API_BASE_URL = config.apiUrl;

export interface ApiKeyInfo {
  id: string;
  name: string;
  key_prefix: string;
  wallet_address: string;
  scopes: string;
  status: string;
  created_at: string;
  last_used_at: string | null;
  expires_at: string | null;
  revoked_at: string | null;
}

export interface CreateApiKeyResponse {
  key: ApiKeyInfo;
  plain_key: string;
}

export interface ListApiKeysResponse {
  keys: ApiKeyInfo[];
}

/**
 * Call the API as the signed-in wallet. The backend identifies the key owner
 * from the SEP-10 session token; it no longer trusts an X-Wallet-Address
 * header, which any client could set to any wallet.
 */
async function fetchWithSession<T>(
  endpoint: string,
  authToken: string,
  options: RequestInit = {},
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;

  const response = await fetch(url, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${authToken}`,
      ...options.headers,
    },
  });

  if (!response.ok) {
    let errorData;
    try {
      errorData = await response.json();
    } catch {
      errorData = { error: response.statusText };
    }
    const { error, message } = (errorData ?? {}) as { error?: string; message?: string };
    throw new Error(message || error || `API error: ${response.status}`);
  }

  return response.json();
}

export async function createApiKey(
  authToken: string,
  name: string,
  scopes?: string,
  expiresAt?: string,
): Promise<CreateApiKeyResponse> {
  return fetchWithSession<CreateApiKeyResponse>("/api/api-keys", authToken, {
    method: "POST",
    body: JSON.stringify({
      name,
      scopes: scopes || "read",
      expires_at: expiresAt || null,
    }),
  });
}

export async function listApiKeys(authToken: string): Promise<ListApiKeysResponse> {
  return fetchWithSession<ListApiKeysResponse>("/api/api-keys", authToken, {
    method: "GET",
  });
}

export async function getApiKey(authToken: string, id: string): Promise<ApiKeyInfo> {
  return fetchWithSession<ApiKeyInfo>(`/api/api-keys/${encodeURIComponent(id)}`, authToken, {
    method: "GET",
  });
}

export async function rotateApiKey(
  authToken: string,
  id: string,
): Promise<CreateApiKeyResponse> {
  return fetchWithSession<CreateApiKeyResponse>(
    `/api/api-keys/${encodeURIComponent(id)}/rotate`,
    authToken,
    { method: "POST" },
  );
}

export async function revokeApiKey(
  authToken: string,
  id: string,
): Promise<{ message: string }> {
  return fetchWithSession<{ message: string }>(
    `/api/api-keys/${encodeURIComponent(id)}`,
    authToken,
    { method: "DELETE" },
  );
}

// ─── Paid plan (USDC on Stellar) ─────────────────────────────────────────────

export interface BillingPlan {
  plan: string;
  price: string;
  asset_code: string;
  asset_issuer: string;
  destination: string;
  period_days: number;
  limit_per_minute: number;
  free_limit_per_minute: number;
  anonymous_limit_per_minute: number;
}

export interface Invoice {
  id: string;
  api_key_id: string;
  amount_usdc: string;
  asset_code: string;
  asset_issuer: string;
  destination: string;
  memo: string;
  limit_per_minute: number;
  period_days: number;
  status: "pending" | "paid" | "expired";
  transaction_hash: string | null;
  created_at: string;
  expires_at: string;
}

export interface Subscription {
  api_key_id: string;
  plan: string;
  limit_per_minute: number;
  paid_until: string;
}

/** The paid plan, or null when this server has paid plans turned off. */
export async function getBillingPlan(): Promise<BillingPlan | null> {
  const response = await fetch(`${API_BASE_URL}/api/billing/plan`);
  if (response.status === 503) return null;
  if (!response.ok) throw new Error(`API error: ${response.status}`);
  return response.json();
}

export async function createInvoice(authToken: string, apiKeyId: string): Promise<Invoice> {
  return fetchWithSession<Invoice>("/api/billing/invoices", authToken, {
    method: "POST",
    body: JSON.stringify({ api_key_id: apiKeyId }),
  });
}

export async function confirmInvoice(
  authToken: string,
  invoiceId: string,
  transactionHash: string,
): Promise<{ invoice: Invoice; subscription: Subscription }> {
  return fetchWithSession(`/api/billing/invoices/${encodeURIComponent(invoiceId)}/confirm`, authToken, {
    method: "POST",
    body: JSON.stringify({ transaction_hash: transactionHash.trim() }),
  });
}

/** The key's paid plan, or null if it has never been upgraded. */
export async function getSubscription(
  authToken: string,
  apiKeyId: string,
): Promise<Subscription | null> {
  try {
    return await fetchWithSession<Subscription>(
      `/api/billing/subscriptions/${encodeURIComponent(apiKeyId)}`,
      authToken,
      { method: "GET" },
    );
  } catch (err) {
    if (err instanceof Error && /no subscription/i.test(err.message)) return null;
    throw err;
  }
}
