/**
 * Anchor metadata from stellar.toml (SEP-1)
 */
export interface AnchorMetadata {
  organization_name?: string;
  organization_dba?: string;
  organization_url?: string;
  organization_logo?: string;
  organization_description?: string;
  organization_support_email?: string;
  supported_currencies?: string[];
  fetched_at?: number;
}

/**
 * Anchor with metrics and metadata
 */
export interface Anchor {
  id: string;
  name: string;
  stellar_account: string;
  reliability_score: number;
  asset_coverage: number;
  failure_rate: number;
  total_transactions: number;
  successful_transactions: number;
  failed_transactions: number;
  status: 'green' | 'yellow' | 'red';
  metadata?: AnchorMetadata;
}

/**
 * Anchors API response
 */
export interface AnchorsResponse {
  anchors: Anchor[];
  total: number;
}
