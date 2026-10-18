import { apiClient } from './client';

export interface ApiKeyScope {
  id: string;
  label: string;
  description?: string;
}

export interface ApiKeyUsage {
  requests: number;
  periodStart: string;
  periodEnd: string;
  quota?: number;
}

export interface ApiKey {
  id: string;
  name: string;
  prefix: string;
  scopes: string[];
  createdAt: string;
  lastUsedAt?: string | null;
  revokedAt?: string | null;
  usage?: ApiKeyUsage | null;
}

export interface CreateApiKeyInput {
  name: string;
  scopes: string[];
}

export interface CreateApiKeyResult {
  key: ApiKey;
  secret: string;
}

export async function listApiKeys(): Promise<ApiKey[]> {
  const { data } = await apiClient.get<ApiKey[]>('/api/v1/api-keys');
  return data;
}

export async function listApiKeyScopes(): Promise<ApiKeyScope[]> {
  const { data } = await apiClient.get<ApiKeyScope[]>('/api/v1/api-keys/scopes');
  return data;
}

export async function createApiKey(input: CreateApiKeyInput): Promise<CreateApiKeyResult> {
  const { data } = await apiClient.post<CreateApiKeyResult>('/api/v1/api-keys', input);
  return data;
}

export async function revokeApiKey(id: string): Promise<ApiKey> {
  const { data } = await apiClient.post<ApiKey>(`/api/v1/api-keys/${encodeURIComponent(id)}/revoke`);
  return data;
}

export async function getApiKeyUsage(id: string): Promise<ApiKeyUsage> {
  const { data } = await apiClient.get<ApiKeyUsage>(`/api/v1/api-keys/${encodeURIComponent(id)}/usage`);
  return data;
}
