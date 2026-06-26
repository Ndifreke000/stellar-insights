import type { HttpClient } from "./http.js";
import type {
  Anchor,
  AlertRule,
  ApiKey,
  AuthTokens,
  ConvertResult,
  CorridorDetail,
  Corridor,
  CostEstimateRequest,
  CostEstimateResponse,
  CreateAlertRuleRequest,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  CreateProposalRequest,
  CreateTransactionRequest,
  CreateWebhookRequest,
  LiquidityPool,
  LoginRequest,
  NetworkInfo,
  PaginatedResponse,
  PaginationParams,
  PaymentPrediction,
  Price,
  Proposal,
  Transaction,
  VerifiedAsset,
  Webhook,
  VoteTally,
  GovernanceProposal,
} from "./types.js";

export class AnchorsResource {
  constructor(private http: HttpClient) {}

  list(params?: PaginationParams): Promise<PaginatedResponse<Anchor>> {
    return this.http.request("GET", "/api/anchors", { params: params as Record<string, unknown> });
  }

  get(id: string): Promise<Anchor> {
    return this.http.request("GET", `/api/anchors/${encodeURIComponent(id)}`);
  }

  getByAccount(account: string): Promise<Anchor> {
    return this.http.request("GET", `/api/anchors/account/${encodeURIComponent(account)}`);
  }
}

export class CorridorsResource {
  constructor(private http: HttpClient) {}

  list(params?: PaginationParams): Promise<PaginatedResponse<Corridor>> {
    return this.http.request("GET", "/api/corridors", { params: params as Record<string, unknown> });
  }

  get(source: string, destination: string): Promise<CorridorDetail> {
    return this.http.request(
      "GET",
      `/api/corridors/${encodeURIComponent(source)}/${encodeURIComponent(destination)}`,
    );
  }
}

export class PricesResource {
  constructor(private http: HttpClient) {}

  list(): Promise<Price[]> {
    return this.http.request("GET", "/api/prices");
  }

  get(asset: string): Promise<Price> {
    return this.http.request("GET", `/api/prices/${encodeURIComponent(asset)}`);
  }

  convert(from: string, to: string, amount: number): Promise<ConvertResult> {
    return this.http.request("POST", "/api/prices/convert", {
      body: { from_asset: from, to_asset: to, amount },
    });
  }
}

export class CostCalculatorResource {
  constructor(private http: HttpClient) {}

  estimate(req: CostEstimateRequest): Promise<CostEstimateResponse> {
    return this.http.request("POST", "/api/cost-calculator/estimate", { body: req });
  }

  routes(req: CostEstimateRequest): Promise<CostEstimateResponse> {
    return this.http.request("POST", "/api/cost-calculator/routes", { body: req });
  }
}

export class AlertsResource {
  constructor(private http: HttpClient) {}

  listRules(params?: PaginationParams): Promise<PaginatedResponse<AlertRule>> {
    return this.http.request("GET", "/api/alerts/rules", { params: params as Record<string, unknown> });
  }

  createRule(req: CreateAlertRuleRequest): Promise<AlertRule> {
    return this.http.request("POST", "/api/alerts/rules", { body: req });
  }

  updateRule(id: string, req: Partial<CreateAlertRuleRequest>): Promise<AlertRule> {
    return this.http.request("PUT", `/api/alerts/rules/${encodeURIComponent(id)}`, { body: req });
  }

  deleteRule(id: string): Promise<void> {
    return this.http.request("DELETE", `/api/alerts/rules/${encodeURIComponent(id)}`);
  }

  listHistory(params?: PaginationParams): Promise<PaginatedResponse<unknown>> {
    return this.http.request("GET", "/api/alerts/history", { params: params as Record<string, unknown> });
  }
}

export class WebhooksResource {
  constructor(private http: HttpClient) {}

  list(): Promise<Webhook[]> {
    return this.http.request("GET", "/api/webhooks");
  }

  create(req: CreateWebhookRequest): Promise<Webhook> {
    return this.http.request("POST", "/api/webhooks", { body: req });
  }

  get(id: string): Promise<Webhook> {
    return this.http.request("GET", `/api/webhooks/${encodeURIComponent(id)}`);
  }

  delete(id: string): Promise<void> {
    return this.http.request("DELETE", `/api/webhooks/${encodeURIComponent(id)}`);
  }

  test(id: string): Promise<unknown> {
    return this.http.request("POST", `/api/webhooks/${encodeURIComponent(id)}/test`);
  }
}

export class ApiKeysResource {
  constructor(private http: HttpClient) {}

  list(): Promise<ApiKey[]> {
    return this.http.request("GET", "/api/api-keys");
  }

  create(req: CreateApiKeyRequest): Promise<CreateApiKeyResponse> {
    return this.http.request("POST", "/api/api-keys", { body: req });
  }

  get(id: string): Promise<ApiKey> {
    return this.http.request("GET", `/api/api-keys/${encodeURIComponent(id)}`);
  }

  rotate(id: string): Promise<CreateApiKeyResponse> {
    return this.http.request("POST", `/api/api-keys/${encodeURIComponent(id)}/rotate`);
  }

  revoke(id: string): Promise<void> {
    return this.http.request("DELETE", `/api/api-keys/${encodeURIComponent(id)}`);
  }
}

export class AuthResource {
  constructor(
    private http: HttpClient,
    private setToken: (token: string) => void,
  ) {}

  async login(req: LoginRequest): Promise<AuthTokens> {
    const tokens = await this.http.request<AuthTokens>("POST", "/api/auth/login", { body: req });
    this.setToken(tokens.access_token);
    return tokens;
  }

  async refresh(refreshToken: string): Promise<AuthTokens> {
    const tokens = await this.http.request<AuthTokens>("POST", "/api/auth/refresh", {
      body: { refresh_token: refreshToken },
    });
    this.setToken(tokens.access_token);
    return tokens;
  }

  logout(): Promise<void> {
    return this.http.request("POST", "/api/auth/logout");
  }
}

export class LiquidityPoolsResource {
  constructor(private http: HttpClient) {}

  list(params?: PaginationParams): Promise<PaginatedResponse<LiquidityPool>> {
    return this.http.request("GET", "/api/liquidity-pools", { params: params as Record<string, unknown> });
  }

  get(id: string): Promise<LiquidityPool> {
    return this.http.request("GET", `/api/liquidity-pools/${encodeURIComponent(id)}`);
  }
}

export class TransactionsResource {
  constructor(private http: HttpClient) {}

  create(req: CreateTransactionRequest): Promise<Transaction> {
    return this.http.request("POST", "/api/transactions", { body: req });
  }

  get(id: string): Promise<Transaction> {
    return this.http.request("GET", `/api/transactions/${encodeURIComponent(id)}`);
  }

  submit(id: string): Promise<Transaction> {
    return this.http.request("POST", `/api/transactions/${encodeURIComponent(id)}/submit`);
  }
}

export class NetworkResource {
  constructor(private http: HttpClient) {}

  info(): Promise<NetworkInfo> {
    return this.http.request("GET", "/api/network");
  }

  available(): Promise<NetworkInfo[]> {
    return this.http.request("GET", "/api/network/available");
  }
}

export class MlResource {
  constructor(private http: HttpClient) {}

  predict(params: Record<string, unknown>): Promise<PaymentPrediction> {
    return this.http.request("POST", "/api/ml/predict", { body: params });
  }

  modelStatus(): Promise<Record<string, unknown>> {
    return this.http.request("GET", "/api/ml/status");
  }
}

export class GovernanceResource {
  constructor(private http: HttpClient) {}

  listProposals(params?: PaginationParams): Promise<PaginatedResponse<Proposal>> {
    return this.http.request("GET", "/api/governance/proposals", { params: params as Record<string, unknown> });
  }

  createProposal(req: CreateProposalRequest): Promise<Proposal> {
    return this.http.request("POST", "/api/governance/proposals", { body: req });
  }

  getProposal(id: string): Promise<Proposal> {
    return this.http.request("GET", `/api/governance/proposals/${encodeURIComponent(id)}`);
  }

  vote(id: string, support: boolean): Promise<VoteTally> {
    return this.http.request("POST", `/api/governance/proposals/${encodeURIComponent(id)}/vote`, {
      body: { support },
    });
  }
}

export class AssetVerificationResource {
  constructor(private http: HttpClient) {}

  verify(assetCode: string, assetIssuer: string): Promise<VerifiedAsset> {
    return this.http.request("POST", "/api/asset-verification/verify", {
      body: { asset_code: assetCode, asset_issuer: assetIssuer },
    });
  }

  get(assetCode: string, assetIssuer: string): Promise<VerifiedAsset> {
    return this.http.request(
      "GET",
      `/api/asset-verification/${encodeURIComponent(assetCode)}/${encodeURIComponent(assetIssuer)}`,
    );
  }

  list(params?: PaginationParams): Promise<PaginatedResponse<VerifiedAsset>> {
    return this.http.request("GET", "/api/asset-verification", { params: params as Record<string, unknown> });
  }
}
