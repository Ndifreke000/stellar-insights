export interface SDKUnitTestsParams {
  suiteName: string;
  timeoutMs?: number;
  metadata?: Record<string, unknown>;
}

export interface SDKUnitTestsResult {
  success: true;
  data: {
    suiteName: string;
    executedAt: string;
    testsRun: number;
    passed: number;
    failed: number;
    metadata?: Record<string, unknown>;
  };
}
