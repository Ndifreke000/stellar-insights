import type { PayRaiderConfig } from "./types.js";
import type {
  NetworkChangeListener,
  NetworkContextManagementParams,
  NetworkContextManagementResult,
  StellarNetwork,
} from "./types/network_context_management.js";
import { EnvironmentDetector } from "./sdk-init.js";
import { SDKError } from "./sdk_error.js";

const NETWORK_URLS: Record<StellarNetwork, string> = {
  mainnet: "https://api.stellarinsights.io",
  testnet: "https://testnet-api.stellarinsights.io",
};

export class NetworkContextManagement {
  public static readonly HEADER_NAME = "X-Stellar-Network";

  private readonly config: PayRaiderConfig;
  private currentNetwork: StellarNetwork;
  private readonly cache = new Map<string, unknown>();
  private readonly listeners = new Set<NetworkChangeListener>();

  constructor(config: PayRaiderConfig = {}) {
    this.config = config;
    this.currentNetwork = NetworkContextManagement.inferNetworkFromConfig(config);
  }

  /**
   * Resolve the active network context, including headers for API requests.
   */
  public async execute(params: NetworkContextManagementParams = {}): Promise<NetworkContextManagementResult> {
    if (params.network !== undefined) {
      NetworkContextManagement.validateNetwork(params.network);
    }

    try {
      const network = params.network ?? this.currentNetwork;
      const baseUrl = this.config.baseUrl ?? NETWORK_URLS[network];
      const environment = EnvironmentDetector.detectEnvironment();

      return {
        success: true,
        network,
        data: {
          baseUrl,
          headers: this.buildHeaders(network),
          environment,
          clientName: params.clientName?.trim() || undefined,
          configuredAt: new Date().toISOString(),
        },
      };
    } catch (error) {
      if (error instanceof SDKError) throw error;
      throw new SDKError("Failed to execute network context management", { params }, { cause: error });
    }
  }

  /** Switch the active network, clear cached data, and notify listeners. */
  public setNetwork(network: StellarNetwork): void {
    NetworkContextManagement.validateNetwork(network);

    const previous = this.currentNetwork;
    if (previous === network) return;

    this.currentNetwork = network;
    this.clearCache();

    for (const listener of this.listeners) {
      listener(network, previous);
    }
  }

  /** Returns the currently active network. */
  public getNetwork(): StellarNetwork {
    return this.currentNetwork;
  }

  /** Headers to attach to every API request for the active network. */
  public getHeaders(network?: StellarNetwork): Record<string, string> {
    const resolved = network ?? this.currentNetwork;
    NetworkContextManagement.validateNetwork(resolved);
    return this.buildHeaders(resolved);
  }

  /** Base URL for the active or specified network. */
  public getBaseUrl(network?: StellarNetwork): string {
    const resolved = network ?? this.currentNetwork;
    NetworkContextManagement.validateNetwork(resolved);
    return this.config.baseUrl ?? NETWORK_URLS[resolved];
  }

  /** Subscribe to network change events. Returns an unsubscribe function. */
  public onNetworkChange(listener: NetworkChangeListener): () => void {
    if (typeof listener !== "function") {
      throw new SDKError("listener must be a function");
    }

    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /** Store a value in the network-scoped cache. */
  public setCacheEntry(key: string, value: unknown): void {
    if (!key || !key.trim()) {
      throw new SDKError("cache key is required");
    }
    this.cache.set(key.trim(), value);
  }

  /** Read a cached value, if present. */
  public getCacheEntry<T = unknown>(key: string): T | undefined {
    return this.cache.get(key.trim()) as T | undefined;
  }

  /** Clear all cached entries (called automatically on network switch). */
  public clearCache(): void {
    this.cache.clear();
  }

  /** Validate a network identifier. */
  public static validateNetwork(network: string): asserts network is StellarNetwork {
    if (network !== "mainnet" && network !== "testnet") {
      throw new SDKError("network must be 'mainnet' or 'testnet'", { network });
    }
  }

  private buildHeaders(network: StellarNetwork): Record<string, string> {
    return {
      [NetworkContextManagement.HEADER_NAME]: network,
    };
  }

  private static inferNetworkFromConfig(config: PayRaiderConfig): StellarNetwork {
    if (config.baseUrl?.includes("testnet")) {
      return "testnet";
    }
    return "mainnet";
  }
}
