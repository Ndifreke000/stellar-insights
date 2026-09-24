import { Platform } from 'react-native';
import { setupNotifications } from './notifications';
import { setupNetworkMonitoring } from './network';
import { loadStoredAuth } from './auth';
import { initializeDatabase } from './database';
import { initializeStorage } from './storage';

export async function initializeApp(): Promise<void> {
  try {
    // Bootstrap MMKV with a device-specific encryption key before any
    // storage read or write (including auth token loading).
    await initializeStorage();

    // Initialize local database
    await initializeDatabase();

    // Load stored authentication
    await loadStoredAuth();

    // Setup network monitoring
    setupNetworkMonitoring();

    // Setup push notifications
    if (Platform.OS !== 'web') {
      await setupNotifications();
    }

    console.log('App initialized successfully');
  } catch (error) {
    console.error('Failed to initialize app:', error);
  }
}
