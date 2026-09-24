import type { PayRaiderConfig } from "./types.js";
import type { ReactNativeCompatibilityParams, ReactNativeCompatibilityResult } from "./types/react_native_compatibility.js";
import { SDKError } from "./sdk_error.js";

export class ReactNativeCompatibility {
  private readonly config: PayRaiderConfig;

  constructor(config: PayRaiderConfig = {}) {
    this.config = config;
  }

  public async execute(params: ReactNativeCompatibilityParams): Promise<ReactNativeCompatibilityResult> {
    if (!params || typeof params.clientName !== "string" || !params.clientName.trim()) {
      throw new SDKError("Invalid React Native compatibility parameters: clientName is required", {
        params,
      });
    }

    try {
      const environment =
        typeof navigator !== "undefined" && (navigator as { product?: string }).product === "ReactNative"
          ? "react-native"
          : "browser";

      return {
        success: true,
        supported: true,
        environment,
        data: {
          clientName: params.clientName.trim(),
          detectedAt: new Date().toISOString(),
          debug: params.debug,
        },
      };
    } catch (error) {
      throw new SDKError("Failed to execute React Native compatibility check", { params, cause: error }, { cause: error });
    }
  }
}
