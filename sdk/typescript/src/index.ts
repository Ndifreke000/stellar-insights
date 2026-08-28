import { HttpClient } from "./http.js";
import {
  AlertsResource,
  AnchorsResource,
  ApiKeysResource,
  AssetVerificationResource,
  AuthResource,
  CorridorsResource,
  CostCalculatorResource,
  GovernanceResource,
  LiquidityPoolsResource,
  MlResource,
  NetworkResource,
  PricesResource,
  TransactionsResource,
  WebhooksResource,
} from "./resources.js";
import type { StellarInsightsConfig } from "./types.js";
import {
  SDKInitializer,
  initializeForMobile,
  initializeForWeb,
  initializeForBackend,
  autoInitialize,
  EnvironmentDetector,
} from "./sdk-init.js";
import { ApiClient, BatchApiClient, ApiClientError } from "./api-client.js";
import {
  acquireConnection,
  releaseConnection,
  closeAllConnections,
  type EventHandler,
} from "./websocket-manager.js";

export interface NetworkConfig {
  rpcUrl: string;
  horizonUrl: string;
  networkPassphrase: string;
  apiBaseUrl: string;
}

export const NETWORKS: Record<"mainnet" | "testnet", NetworkConfig> = {
  mainnet: {
    rpcUrl: "https://stellar.api.onfinality.io/public",
    horizonUrl: "https://horizon.stellar.org",
    networkPassphrase: "Public Global Stellar Network ; September 2015",
    apiBaseUrl: "https://api.stellarinsights.io",
  },
  testnet: {
    rpcUrl: "https://soroban-testnet.stellar.org",
    horizonUrl: "https://horizon-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
    apiBaseUrl: "https://testnet-api.stellarinsights.io",
  },
};

export function createClient(
  network: "mainnet" | "testnet",
  config: Omit<StellarInsightsConfig, "baseUrl"> = {},
): StellarInsights {
  const networkConfig = NETWORKS[network];
  const client = new StellarInsights({
    ...config,
    baseUrl: networkConfig.apiBaseUrl,
  });
  client.wsNetwork = network;
  return client;
}

export class StellarInsights {
  readonly anchors: AnchorsResource;
  readonly corridors: CorridorsResource;
  readonly prices: PricesResource;
  readonly costCalculator: CostCalculatorResource;
  readonly alerts: AlertsResource;
  readonly webhooks: WebhooksResource;
  readonly apiKeys: ApiKeysResource;
  readonly auth: AuthResource;
  readonly liquidityPools: LiquidityPoolsResource;
  readonly transactions: TransactionsResource;
  readonly network: NetworkResource;
  readonly ml: MlResource;
  readonly governance: GovernanceResource;
  readonly assetVerification: AssetVerificationResource;
  readonly apiClient: ApiClient;

  private readonly http: HttpClient;
  private readonly eventHandlers = new Set<EventHandler>();
  /** @internal */
  wsNetwork: "mainnet" | "testnet" = "testnet";

  constructor(config: StellarInsightsConfig = {}) {
    this.http = new HttpClient(config);
    this.apiClient = new ApiClient(config);
    this.anchors = new AnchorsResource(this.http);
    this.corridors = new CorridorsResource(this.http);
    this.prices = new PricesResource(this.http);
    this.costCalculator = new CostCalculatorResource(this.http);
    this.alerts = new AlertsResource(this.http);
    this.webhooks = new WebhooksResource(this.http);
    this.apiKeys = new ApiKeysResource(this.http);
    this.auth = new AuthResource(this.http, (t) => this.http.setToken(t));
    this.liquidityPools = new LiquidityPoolsResource(this.http);
    this.transactions = new TransactionsResource(this.http);
    this.network = new NetworkResource(this.http);
    this.ml = new MlResource(this.http);
    this.governance = new GovernanceResource(this.http);
    this.assetVerification = new AssetVerificationResource(this.http);
  }

  subscribe(handler: EventHandler): void {
    this.eventHandlers.add(handler);
    const net = NETWORKS[this.wsNetwork];
    acquireConnection({ rpcUrl: net.rpcUrl, network: this.wsNetwork }, handler);
  }

  unsubscribe(handler: EventHandler): void {
    this.eventHandlers.delete(handler);
    const net = NETWORKS[this.wsNetwork];
    releaseConnection({ rpcUrl: net.rpcUrl, network: this.wsNetwork }, handler);
  }

  disconnect(): void {
    const net = NETWORKS[this.wsNetwork];
    for (const handler of this.eventHandlers) {
      releaseConnection({ rpcUrl: net.rpcUrl, network: this.wsNetwork }, handler);
    }
    this.eventHandlers.clear();
  }
}

export { StellarInsightsError } from "./http.js";
export { SDKError } from "./sdk_error.js";
export { SDKUnitTests } from "./sdk_unit_tests.js";
export { ReactNativeCompatibility } from "./react_native_compatibility.js";
export { NPMPublishingSetup } from "./npm_publishing_setup.js";
export { AnalyticsAPIModule } from "./analytics_api_module.js";
export { TypeScriptTypes } from "./typescript_types.js";
export { RequestCancellation } from "./request_cancellation.js";
export { RequestDeduplication } from "./request_deduplication.js";
export { NetworkContextManagement } from "./network_context_management.js";
export { AuthenticationModule } from "./authentication_module.js";
export { RetryWithBackoff } from "./retry_with_backoff.js";
export { AnchorsAPIModule } from "./anchors_api_module.js";

// SDK Initialization exports
export { SDKInitializer, initializeForMobile, initializeForWeb, initializeForBackend, autoInitialize, EnvironmentDetector };

// API Client Core exports
export { ApiClient, BatchApiClient, ApiClientError };

// WebSocket exports
export { closeAllConnections };
export type { EventHandler } from "./websocket-manager.js";

export type * from "./types.js";
export type * from "./api-client.js";
export type * from "./types/sdk_unit_tests.js";
export type * from "./types/react_native_compatibility.js";
export type * from "./types/npm_publishing_setup.js";
export type * from "./types/analytics_api_module.js";
export type * from "./types/typescript_types.js";
export type * from "./types/request_cancellation.js";
export type * from "./types/request_deduplication.js";
export type * from "./types/network_context_management.js";
export type * from "./types/authentication_module.js";
export type * from "./types/retry_with_backoff.js";
export type * from "./types/anchors_api_module.js";

// Soroban contract types
export type {
  ProposalStatus,
  VoteChoice,
  GovernanceProposal,
  VoteTally,
  ParameterAction,
  PublicMetadata,
  ContractInfo,
  Snapshot,
  SnapshotMetadata,
} from "./types.js";
