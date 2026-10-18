import notifee, { AndroidImportance } from '@notifee/react-native';
import {
  AuthorizationStatus,
  getMessaging,
  getToken,
  onMessage,
  onTokenRefresh,
  requestPermission,
  type RemoteMessage,
} from '@react-native-firebase/messaging';
import { Platform } from 'react-native';
import { z } from 'zod';
import { apiClient } from './api';
import { useAuthStore } from '@store/authStore';
import { logger } from '@utils/logger';

// In-memory cache for the registered push token
let activeDeviceToken: string | null = null;

/**
 * Returns the currently cached or resolved FCM device token.
 */
export function getActiveDeviceToken(): string | null {
  return activeDeviceToken;
}

// Schema for the notification payload we expect from the backend.
// Any field absent or of the wrong type causes the message to be dropped
// rather than crashing the handler.
const NotificationPayloadSchema = z.object({
  notification: z
    .object({
      title: z.string().optional(),
      body: z.string().optional(),
    })
    .optional(),
  data: z.record(z.string()).optional(),
});

type ValidatedPayload = z.infer<typeof NotificationPayloadSchema>;

function parsePayload(remoteMessage: RemoteMessage): ValidatedPayload | null {
  const result = NotificationPayloadSchema.safeParse(remoteMessage);
  if (!result.success) {
    logger.warn(
      '[Notifications]',
      'Dropping malformed payload:',
      result.error.flatten(),
    );
    return null;
  }
  return result.data;
}

/**
 * Registers the device push token with the backend.
 * Uses the provided token or resolves the current FCM token, and associates
 * it with the authenticated user's Stellar account public key.
 */
export async function registerDeviceTokenWithBackend(
  token?: string,
  account?: string,
): Promise<boolean> {
  try {
    const messagingInstance = getMessaging();
    const resolvedToken = token || (await getToken(messagingInstance));
    if (!resolvedToken || resolvedToken.trim() === '') {
      logger.warn('[Notifications]', 'No device token available to register');
      return false;
    }

    activeDeviceToken = resolvedToken;

    const userAccount = account || useAuthStore.getState().user?.publicKey;
    if (!userAccount) {
      logger.info(
        '[Notifications]',
        'User not authenticated; token cached locally for post-login registration',
      );
      return false;
    }

    await apiClient.post('/notifications/devices', {
      account: userAccount,
      device_token: resolvedToken,
      provider: 'fcm',
      device_label: Platform.OS === 'ios' ? 'iOS' : 'Android',
    });

    logger.info('[Notifications]', 'Device token registered with backend successfully');
    return true;
  } catch (error) {
    logger.warn('[Notifications]', 'Failed to register device token with backend:', error);
    return false;
  }
}

/**
 * Deregisters the device push token from the backend (called on logout).
 */
export async function deregisterDeviceTokenFromBackend(
  token?: string,
): Promise<boolean> {
  try {
    const tokenToRemove = token || activeDeviceToken;
    if (!tokenToRemove) {
      return true;
    }

    await apiClient.delete(
      `/notifications/devices/${encodeURIComponent(tokenToRemove)}`,
    );

    activeDeviceToken = null;
    logger.info('[Notifications]', 'Device token deregistered from backend');
    return true;
  } catch (error) {
    logger.warn('[Notifications]', 'Failed to deregister device token from backend:', error);
    return false;
  }
}

export async function setupNotifications(): Promise<void> {
  // v26's modular API replaces the old messaging()-callable/namespace
  // pattern: an explicit Messaging instance is obtained once and passed to
  // each free function, rather than each call implicitly resolving one.
  const messagingInstance = getMessaging();

  const authStatus = await requestPermission(messagingInstance);
  const enabled =
    authStatus === AuthorizationStatus.AUTHORIZED ||
    authStatus === AuthorizationStatus.PROVISIONAL;

  if (!enabled) {
    logger.info('[Notifications]', 'Permission denied — push notifications will not be shown.');
    return;
  }

  // Obtain device token and register with backend
  const token = await getToken(messagingInstance);
  logger.info('[Notifications]', 'FCM token obtained');
  await registerDeviceTokenWithBackend(token);

  // Listen for token refreshes and re-register
  onTokenRefresh(messagingInstance, async newToken => {
    logger.info('[Notifications]', 'FCM token refreshed');
    await registerDeviceTokenWithBackend(newToken);
  });

  if (Platform.OS === 'android') {
    await notifee.createChannel({
      id: 'default',
      name: 'Default Channel',
      importance: AndroidImportance.HIGH,
    });
  }

  onMessage(messagingInstance, async remoteMessage => {
    const payload = parsePayload(remoteMessage);
    if (!payload) {
      return;
    }

    await notifee.displayNotification({
      title: payload.notification?.title,
      body: payload.notification?.body,
      android: {
        channelId: 'default',
        smallIcon: 'ic_launcher',
      },
    });
  });
}
