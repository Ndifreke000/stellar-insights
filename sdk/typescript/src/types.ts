export interface PreflightCheck {
  name: string;
  passed: boolean;
  detail: string;
  code?: PreflightReasonCode;
}

export type PreflightReasonCode =
  | 'LOW_SUCCESS_RATE'
  | 'THIN_LIQUIDITY'
  | 'SMALL_SAMPLE'
  | 'UNKNOWN_CORRIDOR';
