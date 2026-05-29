import { HttpClient } from "./http.js";
import { AnchorsResource } from "./resources.js";
import type { StellarInsightsConfig } from "./types.js";
import type { AnchorsAPIModuleParams, AnchorsAPIModuleResult } from "./types/anchors_api_module.js";
import { SDKError } from "./sdk_error.js";

export class AnchorsAPIModule {
  private readonly anchors: AnchorsResource;

  constructor(config: StellarInsightsConfig = {}) {
    this.anchors = new AnchorsResource(new HttpClient(config));
  }

  public async execute(params: AnchorsAPIModuleParams): Promise<AnchorsAPIModuleResult> {
    if (!params || !["list", "get", "getByAccount"].includes(params.action)) {
      throw new SDKError('action must be "list", "get", or "getByAccount"', { params });
    }
    if (params.action === "get" && !params.anchorId?.trim()) {
      throw new SDKError("anchorId is required for action \"get\"", { params });
    }
    if (params.action === "getByAccount" && !params.account?.trim()) {
      throw new SDKError("account is required for action \"getByAccount\"", { params });
    }

    try {
      const executedAt = new Date().toISOString();

      if (params.action === "get") {
        const anchor = await this.anchors.get(params.anchorId!.trim());
        return { success: true, action: "get", data: { anchor, executedAt } };
      }

      if (params.action === "getByAccount") {
        const anchor = await this.anchors.getByAccount(params.account!.trim());
        return { success: true, action: "getByAccount", data: { anchor, executedAt } };
      }

      // list
      const response = await this.anchors.list({ page: params.page, limit: params.limit });
      return {
        success: true,
        action: "list",
        data: { anchors: response.data, total: response.pagination.total, executedAt },
      };
    } catch (error) {
      if (error instanceof SDKError) throw error;
      throw new SDKError("Failed to execute anchors API module", { params }, { cause: error });
    }
  }
}
