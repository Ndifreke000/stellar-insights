// `navigator.geolocation` is not part of React Native's core `Navigator` type
// (the polyfill was extracted into `@react-native-community/geolocation`,
// which isn't installed in this project) but src/hooks/useMapsIntegration.ts
// calls it directly and its test mocks `global.navigator.geolocation`
// expecting it to exist. Declare the shape actually used so the code
// type-checks; this does NOT mean the API is available at runtime unless
// something else in the native environment polyfills it.
interface Navigator {
  geolocation: {
    getCurrentPosition(
      success: (position: {
        coords: {
          latitude: number;
          longitude: number;
        };
      }) => void,
      error?: (error: { message: string }) => void,
      options?: {
        enableHighAccuracy?: boolean;
        timeout?: number;
        maximumAge?: number;
      },
    ): void;
  };
}
