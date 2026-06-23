import Config from 'react-native-config';

if (!Config.API_BASE_URL) {
  throw new Error(
    'API_BASE_URL is not set. Copy mobile/.env.example to mobile/.env and set API_BASE_URL to your backend URL.',
  );
}

export const API_CONFIG = {
  BASE_URL: Config.API_BASE_URL,
  TIMEOUT: parseInt(Config.API_TIMEOUT || '30000', 10),
  RETRY_ATTEMPTS: 3,
  RETRY_DELAY: 1000,
} as const;

export const STELLAR_CONFIG = {
  NETWORK: (Config.STELLAR_NETWORK || 'testnet') as 'testnet' | 'mainnet',
  HORIZON_URL: Config.STELLAR_HORIZON_URL || 'https://horizon-testnet.stellar.org',
} as const;

export const STORAGE_KEYS = {
  AUTH_TOKEN: '@auth_token',
  REFRESH_TOKEN: '@refresh_token',
  USER_DATA: '@user_data',
  NETWORK_PREFERENCE: '@network_preference',
  THEME_PREFERENCE: '@theme_preference',
  BIOMETRIC_ENABLED: '@biometric_enabled',
} as const;

export const CACHE_KEYS = {
  CORRIDORS: 'corridors',
  ANCHORS: 'anchors',
  ASSETS: 'assets',
  ANALYTICS: 'analytics',
} as const;

export const FEATURE_FLAGS = {
  BIOMETRIC_AUTH: Config.ENABLE_BIOMETRIC_AUTH === 'true',
  PUSH_NOTIFICATIONS: Config.ENABLE_PUSH_NOTIFICATIONS === 'true',
  OFFLINE_MODE: Config.ENABLE_OFFLINE_MODE === 'true',
} as const;
