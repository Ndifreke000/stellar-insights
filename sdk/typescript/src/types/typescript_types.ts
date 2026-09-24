export interface TypeScriptTypesParams {
  /** Name of the type definition set to validate or describe */
  typeName: string;
  /** Whether to include detailed field-level metadata */
  verbose?: boolean;
  /** Additional metadata to attach to the result */
  metadata?: Record<string, unknown>;
}

export interface TypeScriptTypesResult {
  success: true;
  data: {
    typeName: string;
    resolvedAt: string;
    fieldCount: number;
    verbose?: boolean;
    metadata?: Record<string, unknown>;
  };
}
