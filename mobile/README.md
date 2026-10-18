# Stellar Insights Mobile

React Native mobile application for Stellar Insights payment analytics.

## Current Status

> **Last verified:** October 2026 — clean checkout, Node v24 / npm v11, macOS.
> Tested on iOS Simulator (iPhone 15, iOS 17) and Android Emulator (Pixel 7, API 34).

### Feature checklist

| Feature | Status | Notes |
|---|---|---|
| `npm install` | ✅ Working | 1 226 packages, no peer-dep conflicts |
| `npm run type-check` | ✅ Passing | 0 TypeScript errors |
| `npm test` | ✅ Passing | All unit tests green |
| `npm run lint` | ⚠️ Crashes | `eslint-plugin-prettier@4` / Prettier v3 API mismatch — tracked separately |
| SEP-10 authentication ([SEP-10](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md)) | ✅ Implemented | Challenge/response flow in `services/sep10.ts` |
| Token persistence | ✅ Secure | Uses `react-native-keychain` (hardware-backed Keystore/Keychain) — **no** AsyncStorage for secrets |
| Biometric auth | ✅ Implemented | `services/biometricService.ts` |
| Network switching (testnet ↔ mainnet) | ✅ Working | Settings → Current Network |
| Offline-first caching | ✅ Working | MMKV + AsyncStorage, offline queue flushed on reconnect |
| Push notifications | ✅ Working | Firebase FCM + Notifee, permission requested at boot |
| Corridor watchlist + push alerts | ✅ Implemented | `services/watchlist.ts` — add/remove corridors, offline-queued sync, push on status change |
| Shared logger (no `console.log` in release) | ✅ Implemented | `utils/logger.ts` — debug/info stripped in release, warn/error forwarded to crash tooling |
| Android simulator tested | ✅ | Pixel 7 API 34 emulator |
| iOS simulator tested | ✅ | iPhone 15 iOS 17 simulator |
| Production build | ⚠️ Not verified | Requires signing certificates and Firebase credentials not present in this environment |
| `navigator.geolocation` (maps) | ❌ Not working | No geolocation package installed; `useMapsIntegration.ts` will throw at runtime — separate issue |

### Known gaps / deferred items

- **ESLint** crashes with `TypeError: prettier.resolveConfig.sync is not a function`. Root cause: `@react-native/eslint-config@0.73.2` pulls in `eslint-plugin-prettier@4.2.5`, which uses the Prettier v2 synchronous API; this repo's top-level `prettier` is v3. Fix: pin `prettier` to `^2` or upgrade `@react-native/eslint-config` to a version that supports Prettier v3. Tracking separately.
- **Geolocation**: `@react-native-community/geolocation` is not installed. `navigator.geolocation` is `undefined` at runtime. Affects `useMapsIntegration.ts`.
- **Production signing**: iOS `.xcarchive` / Android `.aab` signing not tested here — no Xcode certificates or Play signing keys in CI.

---

## Features

- 📱 Cross-platform (iOS & Android)
- 🔐 Secure authentication with SEP-10 ([Stellar Ecosystem Proposal 10](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md))
- 🌐 Network switching (testnet/mainnet)
- 📴 Offline-first architecture
- 🔔 Push notifications with corridor watchlist alerts
- 🔒 Biometric authentication
- 🎨 Native UI patterns
- 📋 Corridor watchlist with configurable alert thresholds

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
│   │   ├── api.ts            # Axios HTTP client with token-refresh interceptor
│   │   ├── auth.ts           # Auth tokens — persisted in Keychain/Keystore
│   │   ├── tokenStorage.ts   # Re-export shim (deprecated — use auth.ts)
│   │   ├── storage.ts        # Encrypted MMKV instance
│   │   ├── network.ts        # NetInfo monitoring + offline-queue flush
│   │   ├── notifications.ts  # FCM / Notifee push setup
│   │   ├── watchlist.ts      # Corridor watchlist + push alert subscription
│   │   ├── database.ts       # Placeholder local DB (SQLite migration pending)
│   │   ├── initialization.ts # App boot sequence
│   │   ├── biometricService.ts # Biometric availability + prompt
│   │   └── sep10.ts          # SEP-10 challenge/response authentication
│   ├── store/            # State management (Zustand)
│   ├── hooks/            # Custom React hooks
│   ├── utils/
│   │   └── logger.ts     # Shared logger — debug/info stripped in release
│   ├── types/            # TypeScript types
│   ├── config/           # App configuration (constants, network)
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
- **MMKV**: Fast encrypted local storage
- **React Native Keychain**: Hardware-backed secure credential storage (Keychain / Keystore)
- **Notifee**: Rich local notifications
- **Firebase Messaging**: FCM push notifications
- **Zod**: Runtime schema validation for notification payloads

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
# Note: currently crashes — see Known gaps above
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
- Write operations are queued in MMKV
- Watchlist add/remove changes are queued and replayed on reconnect
- Automatic sync when connection is restored

## Push Notifications & Corridor Watchlist

### Corridor watchlist

Users can watch payment corridors and receive push alerts on status changes.

```
Add corridor:    services/watchlist.ts → addToWatchlist('USDC:NGN', 'any_change')
Remove corridor: services/watchlist.ts → removeFromWatchlist('USDC:NGN')
```

Alert thresholds:
| Threshold | Trigger |
|---|---|
| `any_change` | Any health/status change |
| `down_only` | Corridor goes down or recovers |
| `health_below_50` | Health score drops below 50 % |

### Firebase setup

Configure Firebase for push notifications:

1. Add `google-services.json` (Android) to `android/app/`
2. Add `GoogleService-Info.plist` (iOS) to `ios/`
3. Set Firebase credentials in `.env`

## Security

### Threat model

| Attack surface | Before | After (issue #2628) |
|---|---|---|
| AsyncStorage secrets | ❌ Readable on rooted/compromised devices | ✅ Migrated to Keychain/Keystore |
| expo-secure-store fallback (Android < API 23) | ❌ Fell back to SharedPreferences (cleartext) | ✅ `react-native-keychain` — hardware Keystore only |
| Source code token leakage | ❌ Hardcoded MMKV encryption key | ✅ Random 32-byte key stored in Keychain |
| Log leakage | ❌ `console.log` in production exposed tokens/state | ✅ `utils/logger.ts` strips debug/info in release |

### Storage summary

| Data type | Storage | Encrypted |
|---|---|---|
| Auth tokens (accessToken, refreshToken) | iOS Keychain / Android Keystore | ✅ Hardware-backed |
| MMKV encryption key | iOS Keychain / Android Keystore | ✅ Hardware-backed |
| User preferences, cache | MMKV | ✅ Random device key |
| API response cache | AsyncStorage | ❌ (non-secret cache data only) |

**NEVER** log `accessToken`, `refreshToken`, Stellar secret seeds (`S...`), or full PII.  Use `logger.warn`/`logger.error` with opaque status strings only.

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
