const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080/api";

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

async function fetchWithWallet<T>(
  endpoint: string,
  walletAddress: string,
  options: RequestInit = {},
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;

  const response = await fetch(url, {
    ...options,
    headers: {
      "Content-Type": "application/json",
      "X-Wallet-Address": walletAddress,
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
    throw new Error(
      (errorData as { error?: string })?.error || `API error: ${response.status}`,
    );
  }

  return response.json();
}

export async function createApiKey(
  walletAddress: string,
  name: string,
  scopes?: string,
  expiresAt?: string,
): Promise<CreateApiKeyResponse> {
  return fetchWithWallet<CreateApiKeyResponse>("/keys", walletAddress, {
    method: "POST",
    body: JSON.stringify({
      name,
      scopes: scopes || "read",
      expires_at: expiresAt || null,
    }),
  });
}

export async function listApiKeys(
  walletAddress: string,
): Promise<ListApiKeysResponse> {
  return fetchWithWallet<ListApiKeysResponse>("/keys", walletAddress, {
    method: "GET",
  });
}

export async function getApiKey(
  walletAddress: string,
  id: string,
): Promise<ApiKeyInfo> {
  return fetchWithWallet<ApiKeyInfo>(`/keys/${id}`, walletAddress, {
    method: "GET",
  });
}

export async function rotateApiKey(
  walletAddress: string,
  id: string,
): Promise<CreateApiKeyResponse> {
  return fetchWithWallet<CreateApiKeyResponse>(
    `/keys/${id}/rotate`,
    walletAddress,
    { method: "POST" },
  );
}

export async function revokeApiKey(
  walletAddress: string,
  id: string,
): Promise<{ message: string }> {
  return fetchWithWallet<{ message: string }>(`/keys/${id}`, walletAddress, {
    method: "DELETE",
  });
}
