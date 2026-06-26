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
