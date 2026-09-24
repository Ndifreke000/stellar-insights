export interface AnalyticsAPIModuleParams {
  event: string;
  payload: Record<string, unknown>;
  metadata?: Record<string, unknown>;
}

export interface AnalyticsAPIModuleResult {
  success: true;
  data: {
    recordedEvent: string;
    timestamp: string;
    properties: Record<string, unknown>;
    metadata?: Record<string, unknown>;
  };
}
