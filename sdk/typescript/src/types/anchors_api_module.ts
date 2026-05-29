import type { Anchor, AnchorMetrics } from "../types.js";

export type AnchorAction = "list" | "get" | "getByAccount";

export interface AnchorsAPIModuleParams {
  action: AnchorAction;
  /** Required for action "get" */
  anchorId?: string;
  /** Required for action "getByAccount" */
  account?: string;
  /** Optional pagination for action "list" */
  page?: number;
  limit?: number;
}

export interface AnchorsAPIModuleResult {
  success: true;
  action: AnchorAction;
  data: {
    anchor?: Anchor;
    anchors?: Anchor[];
    total?: number;
    executedAt: string;
  };
}
