import type { PayRaiderConfig } from "./types.js";
import type { AnalyticsAPIModuleParams, AnalyticsAPIModuleResult } from "./types/analytics_api_module.js";
import { SDKError } from "./sdk_error.js";

export class AnalyticsAPIModule {
  private readonly config: PayRaiderConfig;

  constructor(config: PayRaiderConfig = {}) {
    this.config = config;
  }

  public async execute(params: AnalyticsAPIModuleParams): Promise<AnalyticsAPIModuleResult> {
    if (!params || typeof params.event !== "string" || !params.event.trim()) {
      throw new SDKError("Invalid analytics event parameters: event is required", { params });
    }

    if (typeof params.payload !== "object" || params.payload === null) {
      throw new SDKError("Invalid analytics payload: payload must be an object", { params });
    }

    try {
      return {
        success: true,
        data: {
          recordedEvent: params.event.trim(),
          timestamp: new Date().toISOString(),
          properties: { ...params.payload },
          metadata: params.metadata,
        },
      };
    } catch (error) {
      throw new SDKError("Failed to execute analytics API module", { params, cause: error }, { cause: error });
    }
  }
}
