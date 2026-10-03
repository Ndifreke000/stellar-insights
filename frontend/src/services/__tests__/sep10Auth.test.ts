import { vi, describe, it, expect, beforeEach } from 'vitest';
import { signMessage } from '@stellar/freighter-api';
import { Sep10AuthService } from '../sep10Auth';

vi.mock('@stellar/freighter-api', () => ({ signMessage: vi.fn() }));

const CLIENT = 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';
const PASSPHRASE = 'Test SDF Network ; September 2015';

// Mock fetch
global.fetch = vi.fn();

describe('Sep10AuthService', () => {
  let service: Sep10AuthService;

  beforeEach(() => {
    service = new Sep10AuthService('http://localhost:8080');
    vi.mocked(global.fetch).mockClear();
  });

  describe('getInfo', () => {
    it('should fetch SEP-10 server information', async () => {
      const mockInfo = {
        authentication_endpoint: '/api/sep10/auth',
        network_passphrase: 'Test SDF Network ; September 2015',
        signing_key: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
        version: '1.0.0',
      };

      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => mockInfo,
      } as Response);

      const result = await service.getInfo();

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/sep10/info'
      );
      expect(result).toEqual(mockInfo);
    });

    it('should throw error on failed request', async () => {
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: false,
      } as Response);

      await expect(service.getInfo()).rejects.toThrow(
        'Failed to fetch SEP-10 info'
      );
    });
  });

  describe('requestChallenge', () => {
    it('should request a challenge transaction', async () => {
      const mockRequest = {
        account: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
        home_domain: 'example.com',
      };

      const mockResponse = {
        transaction: 'base64encodedxdr',
        network_passphrase: 'Test SDF Network ; September 2015',
      };

      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse,
      } as Response);

      const result = await service.requestChallenge(mockRequest);

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/sep10/auth',
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(mockRequest),
        }
      );
      expect(result).toEqual(mockResponse);
    });

    it('should throw error on failed challenge request', async () => {
      const mockRequest = {
        account: 'INVALID',
      };

      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: false,
        json: async () => ({ error: 'Invalid account' }),
      } as Response);

      await expect(service.requestChallenge(mockRequest)).rejects.toThrow(
        'Invalid account'
      );
    });
  });

  describe('verifyChallenge', () => {
    it('should send the challenge and its signature', async () => {
      const challenge = 'base64challenge';
      const signature = 'base64signature';
      const mockResponse = {
        token: 'jwt-token',
        expires_in: 604800,
      };

      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse,
      } as Response);

      const result = await service.verifyChallenge(challenge, signature);

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/sep10/verify',
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            transaction: challenge,
            signature,
          }),
        }
      );
      expect(result).toEqual(mockResponse);
    });

    it('should throw error on failed verification', async () => {
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: false,
        json: async () => ({ error: 'Invalid signature' }),
      } as Response);

      await expect(service.verifyChallenge('challenge', 'bad-signature')).rejects.toThrow(
        'Invalid signature'
      );
    });
  });

  describe('logout', () => {
    it('should logout and invalidate session', async () => {
      const mockToken = 'jwt-token';

      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ message: 'Logged out successfully' }),
      } as Response);

      await service.logout(mockToken);

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/sep10/logout',
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${mockToken}`,
          },
        }
      );
    });

    it('should throw error on failed logout', async () => {
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: false,
        json: async () => ({ error: 'Session not found' }),
      } as Response);

      await expect(service.logout('invalid-token')).rejects.toThrow(
        'Session not found'
      );
    });
  });

  describe('signChallenge', () => {
    beforeEach(() => {
      vi.mocked(signMessage).mockReset();
    });

    it('signs the challenge as a message for the requested account', async () => {
      vi.mocked(signMessage).mockResolvedValue({ signedMessage: 'c2lnbmF0dXJl', signerAddress: CLIENT });

      const result = await service.signChallenge('challenge', PASSPHRASE, CLIENT);

      expect(result).toBe('c2lnbmF0dXJl');
      expect(signMessage).toHaveBeenCalledWith('challenge', {
        networkPassphrase: PASSPHRASE,
        address: CLIENT,
      });
    });

    it('base64-encodes a Buffer signature from older Freighter versions', async () => {
      vi.mocked(signMessage).mockResolvedValue({
        signedMessage: Buffer.from('signature'),
        signerAddress: CLIENT,
      });

      expect(await service.signChallenge('challenge', PASSPHRASE, CLIENT)).toBe(
        Buffer.from('signature').toString('base64'),
      );
    });

    it('throws when the wallet refuses or is unavailable', async () => {
      vi.mocked(signMessage).mockResolvedValue({
        signedMessage: null,
        signerAddress: '',
        error: { code: -4, message: 'User declined access' },
      });

      await expect(service.signChallenge('challenge', PASSPHRASE, CLIENT)).rejects.toThrow(
        'User declined access',
      );
    });

    it('rejects a signature from a different account', async () => {
      vi.mocked(signMessage).mockResolvedValue({
        signedMessage: 'c2lnbmF0dXJl',
        signerAddress: 'GBOTHERACCOUNTXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
      });

      await expect(service.signChallenge('challenge', PASSPHRASE, CLIENT)).rejects.toThrow(
        'different account',
      );
    });
  });

  describe('validateChallengeTransaction', () => {
    it('should validate a proper challenge transaction', () => {
      const result = service.validateChallengeTransaction(
        'invalid-xdr',
        'GSERVER',
        'Test SDF Network ; September 2015',
        'example.com',
        'GCLIENT'
      );

      // Should return false for invalid XDR
      expect(result).toBe(false);
    });
  });

  describe('authenticate', () => {
    it('should complete full authentication flow', async () => {
      const mockInfo = {
        authentication_endpoint: '/api/sep10/auth',
        network_passphrase: 'Test SDF Network ; September 2015',
        signing_key: 'GSERVER',
        version: '1.0.0',
      };

      const mockChallenge = {
        transaction: 'challenge-xdr',
        network_passphrase: 'Test SDF Network ; September 2015',
      };

      const mockVerification = {
        token: 'jwt-token',
        expires_in: 604800,
      };

      // Mock getInfo
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => mockInfo,
      } as Response);

      // Mock requestChallenge
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => mockChallenge,
      } as Response);

      // Mock wallet signing
      vi.mocked(signMessage).mockResolvedValue({ signedMessage: 'c2lnbmF0dXJl', signerAddress: CLIENT });

      // Mock verifyChallenge
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => mockVerification,
      } as Response);

      const result = await service.authenticate(
        CLIENT,
        {
          homeDomain: 'example.com',
        }
      );

      expect(result).toEqual(mockVerification);
      expect(global.fetch).toHaveBeenCalledTimes(3);
      const [, verifyInit] = vi.mocked(global.fetch).mock.calls[2];
      expect(JSON.parse(String(verifyInit?.body))).toEqual({
        transaction: 'challenge-xdr',
        signature: 'c2lnbmF0dXJl',
      });
    });
  });
});
