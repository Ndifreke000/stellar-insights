// Export Types for Type Safety

export interface ExportColumn {
  id: string;
  label: string;
}

export interface DateRange {
  start: Date | null;
  end: Date | null;
}

// Generic type for exportable data - must have string keys
export type ExportableData = Record<string, any>;

// More specific types for known export data structures
export interface AnalyticsExportData {
  date: string | Date;
  success_rate?: number;
  total_volume?: number;
  tvl?: number;
  latency?: number;
  [key: string]: any; // Allow additional fields
}