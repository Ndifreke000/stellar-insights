export interface ReactNativeCompatibilityParams {
  clientName: string;
  debug?: boolean;
}

export interface ReactNativeCompatibilityResult {
  success: true;
  environment: "react-native" | "browser";
  supported: boolean;
  data: {
    clientName: string;
    detectedAt: string;
    debug?: boolean;
  };
}
