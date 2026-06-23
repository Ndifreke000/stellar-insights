import notifee, { AndroidImportance } from '@notifee/react-native';
import messaging, { FirebaseMessagingTypes } from '@react-native-firebase/messaging';
import { Platform } from 'react-native';
import { z } from 'zod';

const NotificationPayloadSchema = z.object({
  notification: z
    .object({
      title: z.string().optional(),
      body: z.string().optional(),
    })
    .optional(),
  data: z.record(z.string()).optional(),
});

type ValidatedMessage = z.infer<typeof NotificationPayloadSchema>;

function parseMessage(
  remoteMessage: FirebaseMessagingTypes.RemoteMessage,
): ValidatedMessage | null {
  const result = NotificationPayloadSchema.safeParse(remoteMessage);
  if (!result.success) {
    console.warn(
      '[notifications] Dropped malformed push payload:',
      result.error.flatten(),
    );
    return null;
  }
  return result.data;
}

async function displayMessage(message: ValidatedMessage): Promise<void> {
  await notifee.displayNotification({
    title: message.notification?.title,
    body: message.notification?.body,
    android: {
      channelId: 'default',
      smallIcon: 'ic_launcher',
    },
  });
}

export async function setupNotifications(): Promise<void> {
  const authStatus = await messaging().requestPermission();
  const enabled =
    authStatus === messaging.AuthorizationStatus.AUTHORIZED ||
    authStatus === messaging.AuthorizationStatus.PROVISIONAL;

  if (!enabled) {
    console.log('[notifications] Permission denied');
    return;
  }

  const token = await messaging().getToken();
  console.log('[notifications] FCM token obtained');
  // TODO: register token with backend via POST /api/v1/push-tokens
  void token;

  if (Platform.OS === 'android') {
    await notifee.createChannel({
      id: 'default',
      name: 'Default Channel',
      importance: AndroidImportance.HIGH,
    });
  }

  // Foreground messages
  messaging().onMessage(async remoteMessage => {
    const message = parseMessage(remoteMessage);
    if (!message) return;
    await displayMessage(message);
  });

  // Background / quit-state messages open the app
  messaging().setBackgroundMessageHandler(async remoteMessage => {
    const message = parseMessage(remoteMessage);
    if (!message) return;
    await displayMessage(message);
  });
}
