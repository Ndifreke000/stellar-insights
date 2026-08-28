import * as SecureStore from 'expo-secure-store';
import { STORAGE_KEYS } from '@config/constants';
import { useAuthStore } from '@store/authStore';
import { AuthTokens, User } from '@app-types/index';
import { apiClient } from './api';
import { storage } from './storage';

const AUTH_TOKENS_KEY = 'com.stellarinsights.auth.tokens';

export async function loadStoredAuth(): Promise<void> {
  try {
    const credentials = await SecureStore.getItemAsync(AUTH_TOKENS_KEY);
    if (credentials) {
      const tokens: AuthTokens = JSON.parse(credentials);
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
  await SecureStore.setItemAsync(AUTH_TOKENS_KEY, JSON.stringify(tokens));
  useAuthStore.getState().setTokens(tokens);
}

export async function clearAuthTokens(): Promise<void> {
  await SecureStore.deleteItemAsync(AUTH_TOKENS_KEY);
  storage.delete(STORAGE_KEYS.USER_DATA);
  useAuthStore.getState().logout();
}

export async function refreshAuthTokens(): Promise<AuthTokens | null> {
  const { tokens } = useAuthStore.getState();
  if (!tokens?.refreshToken) return null;

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
