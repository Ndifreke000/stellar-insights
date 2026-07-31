/**
 * The subset of @stellar-insights/sdk's StellarInsights client this package
 * depends on. @stellar-insights/sdk does not currently ship its own .d.ts
 * (see src/types/stellar-insights-sdk.d.ts), so this interface stands in for
 * it rather than typing everything as `any`.
 */
export interface PaginationParams {
  page?: number;
  limit?: number;
  sort?: string;
  order?: "asc" | "desc";
}

export interface StellarInsightsClient {
  corridors: {
    list(params?: PaginationParams): Promise<unknown>;
    get(source: string, destination: string): Promise<unknown>;
  };
  anchors: {
    list(params?: PaginationParams): Promise<unknown>;
    get(id: string): Promise<unknown>;
    getByAccount(account: string): Promise<unknown>;
  };
  prices: {
    list(): Promise<unknown>;
    get(asset: string): Promise<unknown>;
    convert(from: string, to: string, amount: number): Promise<unknown>;
  };
  costCalculator: {
    estimate(req: { source_asset: string; destination_asset: string; amount: number }): Promise<unknown>;
  };
  liquidityPools: {
    list(params?: PaginationParams): Promise<unknown>;
    get(id: string): Promise<unknown>;
  };
  network: {
    info(): Promise<unknown>;
    available(): Promise<unknown>;
  };
  ml: {
    predict(params: Record<string, unknown>): Promise<unknown>;
    modelStatus(): Promise<unknown>;
  };
  governance: {
    listProposals(params?: PaginationParams): Promise<unknown>;
    getProposal(id: string): Promise<unknown>;
    createProposal(req: { title: string; description: string }): Promise<unknown>;
    vote(id: string, support: boolean): Promise<unknown>;
  };
  alerts: {
    listHistory(params?: PaginationParams): Promise<unknown>;
    listRules(params?: PaginationParams): Promise<unknown>;
    createRule(req: { name: string; condition: string; threshold: number }): Promise<unknown>;
    updateRule(id: string, req: Partial<{ name: string; condition: string; threshold: number }>): Promise<unknown>;
    deleteRule(id: string): Promise<unknown>;
  };
  assetVerification: {
    verify(assetCode: string, assetIssuer: string): Promise<unknown>;
    get(assetCode: string, assetIssuer: string): Promise<unknown>;
    list(params?: PaginationParams): Promise<unknown>;
  };
  transactions: {
    create(req: { envelope_xdr: string }): Promise<unknown>;
    submit(id: string): Promise<unknown>;
  };
  webhooks: {
    list(): Promise<unknown>;
    create(req: { url: string; events: string[] }): Promise<unknown>;
    get(id: string): Promise<unknown>;
    delete(id: string): Promise<unknown>;
    test(id: string): Promise<unknown>;
  };
  apiKeys: {
    list(): Promise<unknown>;
    create(req: { name: string }): Promise<unknown>;
    get(id: string): Promise<unknown>;
    rotate(id: string): Promise<unknown>;
    revoke(id: string): Promise<unknown>;
  };
}
