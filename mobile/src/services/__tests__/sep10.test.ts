import {
  getServerInfo,
  requestChallenge,
  verifyChallenge,
  logout,
  Sep10Error,
} from '../sep10';
import { apiClient } from '../api';
import { storeAuthTokens, clearAuthTokens } from '../auth';

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

jest.mock('../api', () => ({
  apiClient: {
    get: jest.fn(),
    post: jest.fn(),
  },
}));

jest.mock('../auth', () => ({
  storeAuthTokens: jest.fn(),
  clearAuthTokens: jest.fn(),
}));

const mockGet = apiClient.get as jest.Mock;
const mockPost = apiClient.post as jest.Mock;
const mockStore = storeAuthTokens as jest.Mock;
const mockClear = clearAuthTokens as jest.Mock;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const VALID_ACCOUNT = 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';

const SERVER_INFO = {
  authentication_endpoint: '/api/sep10/auth',
  network_passphrase: 'Test SDF Network ; September 2015',
  signing_key: VALID_ACCOUNT,
  version: '1.0.0',
};

const CHALLENGE_RESPONSE = {
  transaction: 'base64encodedchallenge==',
  network_passphrase: 'Test SDF Network ; September 2015',
};

const VERIFICATION_RESPONSE = {
  token: 'session-token-abc',
  expires_in: 604800, // 7 days in seconds
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

beforeEach(() => {
  jest.clearAllMocks();
  mockStore.mockResolvedValue(undefined);
  mockClear.mockResolvedValue(undefined);
});

describe('getServerInfo', () => {
  it('returns server info on success', async () => {
    mockGet.mockResolvedValue(SERVER_INFO);

    const result = await getServerInfo();

    expect(mockGet).toHaveBeenCalledWith('/api/sep10/info');
    expect(result).toEqual(SERVER_INFO);
  });

  it('wraps API error in Sep10Error', async () => {
    mockGet.mockRejectedValue(makeAxiosError(500, 'Internal error'));

    await expect(getServerInfo()).rejects.toBeInstanceOf(Sep10Error);
    await expect(getServerInfo()).rejects.toMatchObject({
      status: 500,
    });
  });
});

describe('requestChallenge', () => {
  it('requests a challenge for a valid account', async () => {
    mockPost.mockResolvedValue(CHALLENGE_RESPONSE);

    const result = await requestChallenge({ account: VALID_ACCOUNT });

    expect(mockPost).toHaveBeenCalledWith('/api/sep10/auth', { account: VALID_ACCOUNT });
    expect(result).toEqual(CHALLENGE_RESPONSE);
  });

  it('throws Sep10Error immediately for invalid account format', async () => {
    await expect(
      requestChallenge({ account: 'INVALID' }),
    ).rejects.toBeInstanceOf(Sep10Error);

    // apiClient.post should not be called
    expect(mockPost).not.toHaveBeenCalled();
  });

  it('includes optional fields in the request body', async () => {
    mockPost.mockResolvedValue(CHALLENGE_RESPONSE);

    await requestChallenge({
      account: VALID_ACCOUNT,
      home_domain: 'payraider.com',
      client_domain: 'mobile.payraider.com',
      memo: '12345',
    });

    expect(mockPost).toHaveBeenCalledWith('/api/sep10/auth', {
      account: VALID_ACCOUNT,
      home_domain: 'payraider.com',
      client_domain: 'mobile.payraider.com',
      memo: '12345',
    });
  });

  it('wraps a 400 API error in Sep10Error with the server message', async () => {
    mockPost.mockRejectedValue(
      makeAxiosError(400, 'Challenge generation failed: Invalid home domain'),
    );

    const err = await requestChallenge({ account: VALID_ACCOUNT }).catch(e => e);
    expect(err).toBeInstanceOf(Sep10Error);
    expect(err.status).toBe(400);
    expect(err.message).toContain('Challenge generation failed');
  });

  it('wraps a rate-limit 429 error', async () => {
    mockPost.mockRejectedValue(makeAxiosError(429, 'Too many requests'));

    const err = await requestChallenge({ account: VALID_ACCOUNT }).catch(e => e);
    expect(err).toBeInstanceOf(Sep10Error);
    expect(err.status).toBe(429);
  });
});

describe('verifyChallenge', () => {
  it('posts the signed transaction and stores tokens on success', async () => {
    mockPost.mockResolvedValue(VERIFICATION_RESPONSE);
    const beforeCall = Date.now();

    const result = await verifyChallenge('signed-transaction==');

    expect(mockPost).toHaveBeenCalledWith('/api/sep10/verify', {
      transaction: 'signed-transaction==',
    });
    expect(result).toEqual(VERIFICATION_RESPONSE);

    // Tokens should have been persisted
    expect(mockStore).toHaveBeenCalledTimes(1);
    const storedTokens = mockStore.mock.calls[0][0];
    expect(storedTokens.accessToken).toBe(VERIFICATION_RESPONSE.token);
    expect(storedTokens.refreshToken).toBe('');
    expect(storedTokens.expiresAt).toBeGreaterThanOrEqual(
      beforeCall + VERIFICATION_RESPONSE.expires_in * 1000,
    );
  });

  it('throws Sep10Error for an empty transaction string', async () => {
    await expect(verifyChallenge('')).rejects.toBeInstanceOf(Sep10Error);
    expect(mockPost).not.toHaveBeenCalled();
  });

  it('wraps a 401 from the server in Sep10Error', async () => {
    mockPost.mockRejectedValue(makeAxiosError(401, 'Verification failed: bad signature'));

    const err = await verifyChallenge('signed==').catch(e => e);
    expect(err).toBeInstanceOf(Sep10Error);
    expect(err.status).toBe(401);
    expect(mockStore).not.toHaveBeenCalled();
  });

  it('wraps an expired challenge 401 in Sep10Error', async () => {
    mockPost.mockRejectedValue(
      makeAxiosError(401, 'Challenge transaction has expired'),
    );

    const err = await verifyChallenge('expired==').catch(e => e);
    expect(err).toBeInstanceOf(Sep10Error);
    expect(err.status).toBe(401);
  });
});

describe('logout', () => {
  it('calls server logout and clears local tokens', async () => {
    mockPost.mockResolvedValue({});

    await logout();

    expect(mockPost).toHaveBeenCalledWith('/api/sep10/logout', {});
    expect(mockClear).toHaveBeenCalledTimes(1);
  });

  it('clears local tokens even if the server request fails', async () => {
    mockPost.mockRejectedValue(new Error('network error'));

    // Should not throw
    await expect(logout()).resolves.toBeUndefined();
    // Local clear must still run
    expect(mockClear).toHaveBeenCalledTimes(1);
  });
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeAxiosError(status: number, serverMessage: string) {
  const err = new Error(serverMessage) as Error & {
    response?: { status: number; data: { error: string } };
  };
  err.response = { status, data: { error: serverMessage } };
  return err;
}
