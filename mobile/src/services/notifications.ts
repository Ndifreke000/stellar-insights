import notifee, { AndroidImportance } from '@notifee/react-native';
import {
  AuthorizationStatus,
  getMessaging,
  getToken,
  onMessage,
  requestPermission,
  type RemoteMessage,
} from '@react-native-firebase/messaging';
import { Platform } from 'react-native';
import { z } from 'zod';

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
    console.warn(
      '[Notifications] Dropping malformed payload:',
      result.error.flatten(),
    );
    return null;
  }
  return result.data;
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
    console.log('[Notifications] Permission denied');
    return;
  }

  const token = await getToken(messagingInstance);
  console.log('[Notifications] FCM token registered');

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
