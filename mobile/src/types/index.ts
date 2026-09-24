export type StellarNetwork = 'testnet' | 'mainnet';

export interface User {
  id: string;
  publicKey: string;
  email?: string;
  createdAt: string;
}

export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
  expiresAt: number;
}

export interface Corridor {
  id: string;
  sourceAsset: string;
  destinationAsset: string;
  volume24h: number;
  successRate: number;
  avgSettlementTime: number;
}

export interface Anchor {
  id: string;
  name: string;
  domain: string;
  assets: string[];
  status: 'active' | 'inactive' | 'degraded';
  uptime: number;
}

export interface Asset {
  code: string;
  issuer: string;
  domain?: string;
  verified: boolean;
}

export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

export interface PaginatedResponse<T> {
  data: T[];
  cursor?: string;
  hasMore: boolean;
}
