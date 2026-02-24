// Corridor Chart Types for Type Safety

export interface ChartDataPoint {
  timestamp: string;
  [corridorId: string]: string | number; // Dynamic keys for corridor IDs
}

export interface SuccessRateDataPoint extends ChartDataPoint {
  // Inherits timestamp and dynamic corridor keys
}

export interface VolumeDataPoint extends ChartDataPoint {
  // Inherits timestamp and dynamic corridor keys
}

export interface SlippageDataPoint extends ChartDataPoint {
  // Inherits timestamp and dynamic corridor keys
}