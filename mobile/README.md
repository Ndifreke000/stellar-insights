# PayRaider Mobile

React Native mobile application for PayRaider payment analytics.

## Current Status

**Correction:** the "Verified in CI" claims that used to be here were not accurate — the only workflow that touches `mobile/` at all is [`.github/workflows/security-scan.yml`](../.github/workflows/security-scan.yml), which runs `npm audit` and nothing else. Nothing installs, type-checks, lints, or builds this package in CI. The status below reflects an actual run on a clean checkout instead.

**Verified (clean checkout, Node v24.18.0 / npm v11.16.0 — package.json only requires `node >= 18`; this is notably newer than the Node 18/20 era React Native 0.73 targeted, worth keeping in mind if something looks version-sensitive):**
- ✅ `npm install` completes cleanly — 1226 packages, no ERESOLVE / peer-dependency conflicts, no native-module-linking errors, `package-lock.json` unchanged from what's committed.

- ✅ `npm run type-check` passes — `tsc --noEmit` is clean (0 errors). Started at 135 errors across 68 files (130 as last recorded here, +5 once the `network.tsx` parse-blocker below was fixed and `tsc` could actually see what was inside it); fixed all of them rather than suppressing. See commit history on this file's changes for the full breakdown, summarized:
  - `src/config/network.ts` contained JSX but had a `.ts` extension, so nothing in the file could parse — renamed to `.tsx`.
  - Added `@types/node` as a devDependency + `"node"` to `tsconfig.json`'s `types` array (the base RN config restricts auto-inclusion to `["react-native", "jest"]` only) — fixed ~21 errors (`NodeJS` namespace, `process`).
  - Renamed the `@types/*` path alias (in `tsconfig.json`, `babel.config.js`, `jest.config.js`, and ~22 import sites) to `@app-types/*` — it collided with TypeScript's reserved handling of the `@types/` npm scope, which refuses runtime imports through that prefix regardless of what the alias points to. Fixed 23 errors.
  - Replaced invalid ARIA-style `accessibilityRole` values (`"status"`, `"main"`, `"region"`, `"complementary"`, `"listitem"` — none are valid React Native `AccessibilityRole`s, RN has no landmark-role concept) across 14 components: `"status"` → `accessibilityLiveRegion="polite"` (RN's actual equivalent for an ARIA live-region status announcement), the rest simply removed (no RN equivalent). Fixed 36 errors.
  - Fixed 7 `await act(async () => { return await ...; })` test helpers whose return value didn't match `act`'s void-returning type signature — switched to assigning a pre-declared `let` inside the callback instead of returning from it (same runtime behavior, correct types).
  - Renamed the removed-in-RNTL-v12 `getByA11yLabel` query to `getByLabelText` (5 files).
  - Replaced a nonexistent `useMMKVStorage` import (5 files) with the real `useMMKVBoolean` API — this was a **real, currently-broken-at-runtime bug**, not just a type issue: the import didn't exist, so these 5 hooks would throw immediately.
  - Installed `@react-native-community/slider` and switched `VideoPlayerComponent.tsx`'s `Slider` import to it — `Slider` was removed from React Native core; the old import resolved to `undefined` at runtime. Another real, currently-broken bug, not just a type issue.
  - Added `subscribeWithSelector` middleware to `useAppStore` (`src/store/appStore.ts`) — `src/hooks/useOfflineCaching.ts` subscribes with a `(selector, listener)` pair that the base zustand store doesn't support; without the middleware, the selector is silently mistaken for the listener and the real listener (the offline-transition caching logic) never fires. **Another real runtime bug**, not just a type issue.
  - Declared an ambient `Navigator.geolocation` type (`src/types/global.d.ts`) matching what `src/hooks/useMapsIntegration.ts` and its test already assume — flagging this one as unresolved at the *runtime* level: no geolocation package (e.g. `@react-native-community/geolocation`) is installed, so `navigator.geolocation` is likely `undefined` in the actual app right now. This ticket only restored type-checking; the underlying feature gap is a separate, larger fix (needs a real package + native linking, unverifiable here — no simulator/EAS credentials in this environment).
  - A handful of one-off fixes: two `.testnet.test.ts` smoke-test files that lacked any `import`/`export` and were colliding as global scripts (added `export {}`); a duplicate `accessibilityLiveRegion` attribute my own `"status"` → `accessibilityLiveRegion` batch fix accidentally introduced in `SyncStatusBanner.tsx` (caught and fixed before committing); a couple of test fixtures missing required fields; a `Platform.Version as string` cast replaced with `String(Platform.Version)`; a few `Record<string, string>` inference mismatches from array literals with per-item varying shapes, fixed with explicit type annotations.
- ❌ `npm run lint` still doesn't run at all — crashes immediately with `TypeError: prettier.resolveConfig.sync is not a function`. Root cause: `@react-native/eslint-config@0.73.2` pulls in its own nested `eslint-plugin-prettier@4.2.5`, built against Prettier's pre-v3 (sync) API; this repo's `prettier` resolves to `3.8.3`. Likely fix is an npm `overrides` pin of `eslint-plugin-prettier` to `^5` (the version compatible with Prettier v3) — not attempted, out of scope for the type-check pass; needs its own full clean lint run to verify it doesn't change lint behavior elsewhere in the package.
- ⚠️ `npm test` (jest) has pre-existing failures unrelated to the above: 23 of 63 suites fail, mostly `Invariant Violation: TurboModuleRegistry.getEnforcing(...): 'SettingsManager' could not be found` — a missing native-module mock in the jest environment, not a code bug. Confirmed via a baseline run before any type-check fixes: identical 23 suites failed before and after (byte-for-byte same suite list), so none of the type-check work caused or masked a test regression. This is a real, separate gap worth its own ticket.

**NOT verified (no simulator/EAS credentials in CI environment):**
- ❌ Native iOS build (simulator or device)
- ❌ Native Android build (emulator or device)
- ❌ End-to-end testing on actual device
- ❌ App store distribution (iOS/Google Play)

**What this means:**
- `npm install` and `npm run type-check` are both reliable now — dependencies land on disk without special flags, and `tsc --noEmit` is a meaningful gate again.
- Don't rely on `npm run lint` or `npm test` as stabilization signals yet; both need dedicated follow-up work (see above) before they're meaningful gates.
- The app can be built and run locally if you have Xcode/Android Studio configured, independent of the lint/test issues above (Metro/Babel transpile without type-checking).
- The geolocation and (fixed) Slider features were using APIs with no installed backing package — geolocation still is. Don't assume every feature under `src/features/`/`src/hooks/` works on a real device without checking its dependencies are actually installed.
- Contributors who want to test native builds must do so on their own machine
- Full testing requires running `npm run ios` or `npm run android` locally

**To verify native builds yourself:**

```bash
# Install dependencies
npm install
cd ios && pod install && cd ..

# For iOS (requires Xcode):
npm run ios

# For Android (requires Android Studio/SDK):
npm run android
```

If you encounter issues, see the Troubleshooting section below. If you fix an issue, please document it in this README or open an issue for the team.

## Features

- 📱 Cross-platform (iOS & Android)
- 🔐 Secure authentication with SEP-10
- 🌐 Network switching (testnet/mainnet)
- 📴 Offline-first architecture
- 🔔 Push notifications
- 🔒 Biometric authentication
- 🎨 Native UI patterns

## Prerequisites

- Node.js 18+
- React Native CLI
- Xcode (for iOS)
- Android Studio (for Android)
- CocoaPods (for iOS)

## Setup

1. Install dependencies:

```bash
npm install
```

2. Install iOS pods:

```bash
cd ios && pod install && cd ..
```

3. Configure environment:

```bash
cp .env.example .env
# Edit .env with your configuration
```

4. Run the app:

```bash
# iOS
npm run ios

# Android
npm run android
```

## Project Structure

```
mobile/
├── src/
│   ├── components/       # Reusable UI components
│   ├── screens/          # Screen components
│   │   ├── auth/         # Authentication screens
│   │   └── main/         # Main app screens
│   ├── navigation/       # Navigation configuration
│   ├── services/         # API and business logic
│   │   ├── api.ts        # API client
│   │   ├── auth.ts       # Authentication service
│   │   ├── storage.ts    # Local storage
│   │   ├── network.ts    # Network monitoring
│   │   └── notifications.ts # Push notifications
│   ├── store/            # State management (Zustand)
│   ├── hooks/            # Custom React hooks
│   ├── utils/            # Utility functions
│   ├── types/            # TypeScript types
│   ├── config/           # App configuration
│   └── App.tsx           # Root component
├── android/              # Android native code
├── ios/                  # iOS native code
└── package.json
```

## Key Dependencies

- **React Native**: Cross-platform framework
- **React Navigation**: Navigation library
- **TanStack Query**: Data fetching and caching
- **Zustand**: State management
- **Axios**: HTTP client
- **MMKV**: Fast local storage
- **React Native Keychain**: Secure credential storage
- **Notifee**: Local notifications
- **Firebase**: Push notifications

## Development

### Running Tests

```bash
npm test
```

### Type Checking

```bash
npm run type-check
```

### Linting

```bash
npm run lint
```

## Network Switching

The app supports runtime network switching between testnet and mainnet:

1. Go to Settings
2. Tap "Current Network"
3. Select desired network
4. App will clear cache and reconnect

## Offline Mode

The app works offline with cached data:

- Cached data is marked with staleness indicators
- Write operations are queued
- Automatic sync when connection is restored

## Push Notifications

Configure Firebase for push notifications:

1. Add `google-services.json` (Android) to `android/app/`
2. Add `GoogleService-Info.plist` (iOS) to `ios/`
3. Set Firebase credentials in `.env`

## Security

- Tokens stored in platform keychain
- Biometric authentication support
- Certificate pinning (production)
- Secure local storage with encryption

## Building for Production

### iOS

```bash
cd ios
xcodebuild -workspace StellarInsights.xcworkspace -scheme StellarInsights -configuration Release
```

### Android

```bash
cd android
./gradlew assembleRelease
```

## Troubleshooting

### Metro bundler issues

```bash
npm start -- --reset-cache
```

### iOS build issues

```bash
cd ios
pod deintegrate
pod install
```

### Android build issues

```bash
cd android
./gradlew clean
```

## Contributing

See main repository CONTRIBUTING.md

## License

See main repository LICENSE
