import * as Keychain from 'react-native-keychain';
import { STORAGE_KEYS } from '@config/constants';
import { useAuthStore } from '@store/authStore';
import { AuthTokens, User } from '@appTypes/index';
import { apiClient } from './api';
import { storage } from './storage';

export async function loadStoredAuth(): Promise<void> {
  try {
    const credentials = await Keychain.getGenericPassword();
    if (credentials) {
      const tokens: AuthTokens = JSON.parse(credentials.password);
      useAuthStore.getState().setTokens(tokens);

      const userData = storage.getString(STORAGE_KEYS.USER_DATA);
      if (userData) {
        const user: User = JSON.parse(userData);
        useAuthStore.getState().setUser(user);
      }
    }
  } catch (error) {
    console.error('Failed to load stored auth:', error);
  } finally {
    useAuthStore.getState().setLoading(false);
  }
}

export async function storeAuthTokens(tokens: AuthTokens): Promise<void> {
  await Keychain.setGenericPassword('auth', JSON.stringify(tokens));
  useAuthStore.getState().setTokens(tokens);
}

export async function clearAuthTokens(): Promise<void> {
  await Keychain.resetGenericPassword();
  storage.delete(STORAGE_KEYS.USER_DATA);
  useAuthStore.getState().logout();
}

export async function refreshAuthTokens(): Promise<AuthTokens | null> {
  const { tokens } = useAuthStore.getState();
  if (!tokens?.refreshToken) {
    return null;
  }

  try {
    const newTokens = await apiClient.post<AuthTokens>('/auth/refresh', {
      refreshToken: tokens.refreshToken,
    });

    await storeAuthTokens(newTokens);
    return newTokens;
  } catch (error) {
    console.error('Failed to refresh tokens:', error);
    return null;
  }
}
