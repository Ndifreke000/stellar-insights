import { vi, describe, it, expect, beforeEach } from 'vitest';
import { signTransaction } from '@stellar/freighter-api';
import { Sep10AuthService } from '../sep10Auth';

vi.mock('@stellar/freighter-api', () => ({ signTransaction: vi.fn() }));

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
    it('posts the signed envelope XDR with no separate signature field', async () => {
      const mockResponse = {
        token: 'jwt-token',
        expires_in: 604800,
      };

      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse,
      } as Response);

      const result = await service.verifyChallenge('signed-envelope-xdr');

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/sep10/verify',
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ transaction: 'signed-envelope-xdr' }),
        }
      );
      expect(result).toEqual(mockResponse);
    });

    it('should throw error on failed verification', async () => {
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: false,
        json: async () => ({ error: 'Invalid signature' }),
      } as Response);

      await expect(service.verifyChallenge('bad')).rejects.toThrow(
        'Invalid signature'
      );
    });
  });

  describe('signChallenge', () => {
    beforeEach(() => {
      vi.mocked(signTransaction).mockReset();
    });

    it('signs the challenge transaction envelope and returns the signed XDR', async () => {
      vi.mocked(signTransaction).mockResolvedValue({
        signedTxXdr: 'signed-envelope-xdr',
        signerAddress: CLIENT,
      } as never);

      const result = await service.signChallenge(
        'challenge-xdr',
        PASSPHRASE,
        CLIENT
      );

      expect(result).toBe('signed-envelope-xdr');
      expect(signTransaction).toHaveBeenCalledWith('challenge-xdr', {
        networkPassphrase: PASSPHRASE,
        address: CLIENT,
      });
    });

    it('throws when the wallet refuses or is unavailable', async () => {
      vi.mocked(signTransaction).mockResolvedValue({
        signedTxXdr: '',
        error: { code: -4, message: 'User declined access' },
      } as never);

      await expect(
        service.signChallenge('challenge-xdr', PASSPHRASE, CLIENT)
      ).rejects.toThrow('User declined access');
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
    it('completes the full XDR flow: challenge -> sign -> verify', async () => {
      const mockInfo = {
        authentication_endpoint: '/api/sep10/auth',
        network_passphrase: PASSPHRASE,
        signing_key: 'GSERVER',
        version: '1.0.0',
      };

      const mockChallenge = {
        transaction: 'challenge-xdr',
        network_passphrase: PASSPHRASE,
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

      // Mock wallet signing of the transaction envelope
      vi.mocked(signTransaction).mockResolvedValue({
        signedTxXdr: 'signed-envelope-xdr',
      } as never);

      // Mock verifyChallenge
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => mockVerification,
      } as Response);

      const result = await service.authenticate(CLIENT, {
        homeDomain: 'example.com',
      });

      expect(result).toEqual(mockVerification);
      expect(global.fetch).toHaveBeenCalledTimes(3);
      const [, verifyInit] = vi.mocked(global.fetch).mock.calls[2];
      expect(JSON.parse(String(verifyInit?.body))).toEqual({
        transaction: 'signed-envelope-xdr',
      });
      expect(signTransaction).toHaveBeenCalledWith('challenge-xdr', {
        networkPassphrase: PASSPHRASE,
        address: CLIENT,
      });
    });
  });

  describe('getSession', () => {
    it('reports authenticated when the server says so', async () => {
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ authenticated: true }),
      } as Response);

      expect(await service.getSession()).toEqual({ authenticated: true });
      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/sep10/session',
        {
          method: 'GET',
          credentials: 'same-origin',
          headers: { Accept: 'application/json' },
        }
      );
    });

    it('reports unauthenticated when the endpoint is missing', async () => {
      vi.mocked(global.fetch).mockResolvedValueOnce({ ok: false } as Response);

      expect(await service.getSession()).toEqual({ authenticated: false });
    });
  });

  describe('logout', () => {
    it('posts a session-authenticated logout with no body', async () => {
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ message: 'Logged out successfully' }),
      } as Response);

      await service.logout();

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/sep10/logout',
        {
          method: 'POST',
          credentials: 'same-origin',
        }
      );
    });

    it('should throw error on failed logout', async () => {
      vi.mocked(global.fetch).mockResolvedValueOnce({
        ok: false,
        json: async () => ({ error: 'Session not found' }),
      } as Response);

      await expect(service.logout()).rejects.toThrow('Session not found');
    });
  });
});
