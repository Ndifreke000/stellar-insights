import {
  setupNotifications,
  registerDeviceTokenWithBackend,
  deregisterDeviceTokenFromBackend,
} from '../notifications';
import { apiClient } from '../api';
import { useAuthStore } from '@store/authStore';
import {
  AuthorizationStatus,
  getMessaging,
  getToken,
  requestPermission,
  onTokenRefresh,
} from '@react-native-firebase/messaging';

jest.mock('../api', () => ({
  apiClient: {
    post: jest.fn(),
    delete: jest.fn(),
  },
}));

jest.mock('@react-native-firebase/messaging', () => {
  const dummyMessaging = {};
  return {
    getMessaging: jest.fn(() => dummyMessaging),
    requestPermission: jest.fn(),
    getToken: jest.fn(),
    onTokenRefresh: jest.fn(),
    onMessage: jest.fn(),
    AuthorizationStatus: {
      AUTHORIZED: 1,
      DENIED: 0,
      NOT_DETERMINED: -1,
      PROVISIONAL: 2,
    },
  };
});

jest.mock('@notifee/react-native', () => ({
  createChannel: jest.fn(),
  displayNotification: jest.fn(),
  AndroidImportance: {
    HIGH: 4,
  },
}));

const mockPost = apiClient.post as jest.Mock;
const mockDelete = apiClient.delete as jest.Mock;
const mockRequestPermission = requestPermission as jest.Mock;
const mockGetToken = getToken as jest.Mock;
const mockOnTokenRefresh = onTokenRefresh as jest.Mock;

const MOCK_ACCOUNT = 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';
const MOCK_TOKEN = 'test-fcm-token-12345';

describe('mobile notifications service', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    useAuthStore.getState().logout();
  });

  it('registers device token with backend when user is authenticated', async () => {
    useAuthStore.getState().setUser({
      id: 'user-1',
      publicKey: MOCK_ACCOUNT,
      createdAt: '2026-01-01',
    });
    mockPost.mockResolvedValueOnce({ success: true });

    const success = await registerDeviceTokenWithBackend(MOCK_TOKEN);
    expect(success).toBe(true);
    expect(mockPost).toHaveBeenCalledWith('/notifications/devices', {
      account: MOCK_ACCOUNT,
      device_token: MOCK_TOKEN,
      provider: 'fcm',
      device_label: expect.any(String),
    });
  });

  it('caches token and defers backend call when user is not authenticated', async () => {
    mockGetToken.mockResolvedValueOnce(MOCK_TOKEN);

    const success = await registerDeviceTokenWithBackend(MOCK_TOKEN);
    expect(success).toBe(false);
    expect(mockPost).not.toHaveBeenCalled();
  });

  it('deregisters device token from backend on logout', async () => {
    mockDelete.mockResolvedValueOnce({ success: true });

    const success = await deregisterDeviceTokenFromBackend(MOCK_TOKEN);
    expect(success).toBe(true);
    expect(mockDelete).toHaveBeenCalledWith(
      `/notifications/devices/${encodeURIComponent(MOCK_TOKEN)}`,
    );
  });

  it('sets up notifications and hooks token refresh listener', async () => {
    mockRequestPermission.mockResolvedValueOnce(AuthorizationStatus.AUTHORIZED);
    mockGetToken.mockResolvedValueOnce(MOCK_TOKEN);
    useAuthStore.getState().setUser({
      id: 'user-1',
      publicKey: MOCK_ACCOUNT,
      createdAt: '2026-01-01',
    });
    mockPost.mockResolvedValue({ success: true });

    await setupNotifications();

    expect(mockRequestPermission).toHaveBeenCalled();
    expect(mockGetToken).toHaveBeenCalled();
    expect(mockPost).toHaveBeenCalledWith('/notifications/devices', {
      account: MOCK_ACCOUNT,
      device_token: MOCK_TOKEN,
      provider: 'fcm',
      device_label: expect.any(String),
    });
    expect(mockOnTokenRefresh).toHaveBeenCalled();
  });

  it('skips token retrieval if permission is denied', async () => {
    mockRequestPermission.mockResolvedValueOnce(AuthorizationStatus.DENIED);

    await setupNotifications();

    expect(mockGetToken).not.toHaveBeenCalled();
    expect(mockPost).not.toHaveBeenCalled();
  });
});
