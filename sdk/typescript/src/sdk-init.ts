import type { StellarInsightsConfig } from "./types.js";

/**
 * SDK initialization configuration with network awareness
 */
export interface SDKInitConfig extends StellarInsightsConfig {
  /** Enable debug logging */
  debug?: boolean;
  /** Target network: mainnet or testnet */
  network?: "mainnet" | "testnet";
  /** Custom base URL for the API */
  baseUrl?: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Maximum number of retries */
  maxRetries?: number;
  /** Initial retry delay in milliseconds */
  retryDelay?: number;
  /** Enable automatic retry */
  autoRetry?: boolean;
  /** Cache timeout in seconds (0 = no cache) */
  cacheTimeout?: number;
  /** Enable request/response logging */
  logRequests?: boolean;
}

/**
 * SDK initialization context
 */
export interface SDKInitContext {
  config: SDKInitConfig;
  isInitialized: boolean;
  isReactNative: boolean;
  environment: "browser" | "node" | "react-native";
  version: string;
}

/**
 * Environment detection utilities
 */
export class EnvironmentDetector {
  static isReactNative(): boolean {
    // Check for React Native runtime
    return (
      typeof navigator !== "undefined" &&
      navigator.userAgent?.includes("React Native") === true
    ) ||
      (typeof global !== "undefined" &&
        (global as any).HermesInternal !== undefined);
  }

  static isBrowser(): boolean {
    return (
      typeof window !== "undefined" &&
      typeof document !== "undefined" &&
      !this.isReactNative()
    );
  }

  static isNode(): boolean {
    return (
      typeof process !== "undefined" &&
      process.versions?.node !== undefined &&
      !this.isBrowser() &&
      !this.isReactNative()
    );
  }

  static detectEnvironment(): "browser" | "node" | "react-native" {
    if (this.isReactNative()) {
      return "react-native";
    }
    if (this.isBrowser()) {
      return "browser";
    }
    if (this.isNode()) {
      return "node";
    }
    return "browser"; // Default fallback
  }

  static getPlatform(): string {
    if (this.isReactNative()) {
      // React Native's `Platform` export isn't a JS global - it comes from
      // the `react-native` package, which this SDK doesn't depend on. Look
      // it up defensively, the same way isReactNative() checks HermesInternal.
      const platform = (globalThis as { Platform?: { OS?: string } }).Platform;
      return platform?.OS ?? "react-native";
    }
    if (this.isBrowser()) {
      return "web";
    }
    return "node";
  }
}

/**
 * Default configuration values
 */
const DEFAULT_CONFIG: SDKInitConfig = {
  debug: false,
  network: "mainnet",
  baseUrl: "https://api.stellarinsights.io",
  timeout: 30000,
  maxRetries: 3,
  retryDelay: 500,
  autoRetry: true,
  cacheTimeout: 60,
  logRequests: false,
};

/**
 * SDK Initializer - Entry point for SDK setup
 */
export class SDKInitializer {
  private static instance: SDKInitContext | null = null;

  /**
   * Initialize the SDK with provided configuration
   */
  static initialize(config: SDKInitConfig = {}): SDKInitContext {
    // Merge with defaults
    const mergedConfig: SDKInitConfig = {
      ...DEFAULT_CONFIG,
      ...config,
    };

    // Network-specific URL override
    if (mergedConfig.network === "testnet" && !config.baseUrl) {
      mergedConfig.baseUrl = "https://testnet-api.stellarinsights.io";
    }

    const environment = EnvironmentDetector.detectEnvironment();
    const isReactNative = EnvironmentDetector.isReactNative();

    // Log initialization if debug mode is enabled
    if (mergedConfig.debug) {
      console.log(
        `[Stellar Insights SDK] Initializing in ${environment} environment`,
        {
          network: mergedConfig.network,
          platform: EnvironmentDetector.getPlatform(),
          isReactNative,
        }
      );
    }

    const context: SDKInitContext = {
      config: mergedConfig,
      isInitialized: true,
      isReactNative,
      environment,
      version: "1.0.0", // Will be replaced during build
    };

    this.instance = context;
    return context;
  }

  /**
   * Get the current initialization context
   */
  static getInstance(): SDKInitContext {
    if (!this.instance) {
      return this.initialize();
    }
    return this.instance;
  }

  /**
   * Check if SDK is properly initialized
   */
  static isInitialized(): boolean {
    return this.instance !== null && this.instance.isInitialized;
  }

  /**
   * Reset the SDK (useful for testing)
   */
  static reset(): void {
    this.instance = null;
  }

  /**
   * Get configuration option
   */
  static getConfig<K extends keyof SDKInitConfig>(
    key: K
  ): SDKInitConfig[K] | undefined {
    const context = this.getInstance();
    return context.config[key];
  }

  /**
   * Update configuration option
   */
  static setConfig<K extends keyof SDKInitConfig>(
    key: K,
    value: SDKInitConfig[K]
  ): void {
    const context = this.getInstance();
    context.config[key] = value;
  }

  /**
   * Enable debug mode
   */
  static enableDebug(): void {
    this.setConfig("debug", true);
  }

  /**
   * Disable debug mode
   */
  static disableDebug(): void {
    this.setConfig("debug", false);
  }

  /**
   * Switch network
   */
  static switchNetwork(network: "mainnet" | "testnet"): void {
    this.setConfig("network", network);
    if (network === "testnet") {
      this.setConfig(
        "baseUrl",
        "https://testnet-api.stellarinsights.io"
      );
    } else {
      this.setConfig("baseUrl", "https://api.stellarinsights.io");
    }
  }

  /**
   * Get platform information
   */
  static getPlatformInfo() {
    return {
      platform: EnvironmentDetector.getPlatform(),
      environment: EnvironmentDetector.detectEnvironment(),
      isReactNative: EnvironmentDetector.isReactNative(),
      isBrowser: EnvironmentDetector.isBrowser(),
      isNode: EnvironmentDetector.isNode(),
    };
  }
}

/**
 * Initialize the SDK for mobile React Native
 */
export function initializeForMobile(config: Partial<SDKInitConfig> = {}): SDKInitContext {
  return SDKInitializer.initialize({
    ...config,
    // Optimize for mobile
    timeout: config.timeout ?? 15000,
    maxRetries: config.maxRetries ?? 3,
    retryDelay: config.retryDelay ?? 1000,
    cacheTimeout: config.cacheTimeout ?? 120,
  });
}

/**
 * Initialize the SDK for web
 */
export function initializeForWeb(config: Partial<SDKInitConfig> = {}): SDKInitContext {
  return SDKInitializer.initialize({
    ...config,
    logRequests: config.logRequests ?? true,
  });
}

/**
 * Initialize the SDK for backend/Node.js
 */
export function initializeForBackend(config: Partial<SDKInitConfig> = {}): SDKInitContext {
  return SDKInitializer.initialize({
    ...config,
    timeout: config.timeout ?? 60000,
    cacheTimeout: config.cacheTimeout ?? 300,
  });
}

/**
 * Auto-initialize based on detected environment
 */
export function autoInitialize(config: Partial<SDKInitConfig> = {}): SDKInitContext {
  const environment = EnvironmentDetector.detectEnvironment();

  switch (environment) {
    case "react-native":
      return initializeForMobile(config);
    case "node":
      return initializeForBackend(config);
    case "browser":
    default:
      return initializeForWeb(config);
  }
}

export default SDKInitializer;
