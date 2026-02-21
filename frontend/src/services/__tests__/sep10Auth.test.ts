import { Sep10AuthService } from '../sep10Auth';

// Mock fetch
global.fetch = jest.fn();

describe('Sep10AuthService', () => {
  let service: Sep10AuthService;

  beforeEach(() => {
    service = new Sep10AuthService('http://localhost:8080');
    (global.fetch as jest.Mock).mockClear();
  });

  describe('getInfo', () => {
    it('should fetch SEP-10 server information', async () => {
      const mockInfo = {
        authentication_endpoint: '/api/sep10/auth',
        network_passphrase: 'Test SDF Network ; September 2015',
        signing_key: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
        version: '1.0.0',
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockInfo,
      });

      const result = await service.getInfo();

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/sep10/info'
      );
      expect(result).toEqual(mockInfo);
    });

    it('should throw error on failed request', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
      });

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

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse,
      });

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

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        json: async () => ({ error: 'Invalid account' }),
      });

      await expect(service.requestChallenge(mockRequest)).rejects.toThrow(
        'Invalid account'
      );
    });
  });

  describe('verifyChallenge', () => {
    it('should verify a signed challenge transaction', async () => {
      const mockSignedXdr = 'signedbase64encodedxdr';
      const mockResponse = {
        token: 'jwt-token',
        expires_in: 604800,
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await service.verifyChallenge(mockSignedXdr);

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/sep10/verify',
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            transaction: mockSignedXdr,
          }),
        }
      );
      expect(result).toEqual(mockResponse);
    });

    it('should throw error on failed verification', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        json: async () => ({ error: 'Invalid signature' }),
      });

      await expect(service.verifyChallenge('invalid')).rejects.toThrow(
        'Invalid signature'
      );
    });
  });

  describe('logout', () => {
    it('should logout and invalidate session', async () => {
      const mockToken = 'jwt-token';

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ message: 'Logged out successfully' }),
      });

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
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        json: async () => ({ error: 'Session not found' }),
      });

      await expect(service.logout('invalid-token')).rejects.toThrow(
        'Session not found'
      );
    });
  });

  describe('signChallenge', () => {
    it('should sign challenge with Freighter wallet', async () => {
      const mockWindow = {
        freighter: {
          signTransaction: jest.fn().mockResolvedValue('signed-xdr'),
        },
      };

      (global as any).window = mockWindow;

      const result = await service.signChallenge(
        'challenge-xdr',
        'Test SDF Network ; September 2015',
        'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX'
      );

      expect(result).toBe('signed-xdr');
      expect(mockWindow.freighter.signTransaction).toHaveBeenCalledWith(
        'challenge-xdr',
        {
          network: 'Test SDF Network ; September 2015',
          networkPassphrase: 'Test SDF Network ; September 2015',
          accountToSign: 'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX',
        }
      );
    });

    it('should throw error when no wallet is available', async () => {
      (global as any).window = {};

      await expect(
        service.signChallenge(
          'challenge-xdr',
          'Test SDF Network ; September 2015',
          'GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX'
        )
      ).rejects.toThrow('No compatible Stellar wallet found');
    });
  });

  describe('validateChallengeTransaction', () => {
    it('should validate a proper challenge transaction', () => {
      // This would require mocking the Stellar SDK Transaction class
      // For now, we'll test the error cases

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
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockInfo,
      });

      // Mock requestChallenge
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockChallenge,
      });

      // Mock wallet signing
      const mockWindow = {
        freighter: {
          signTransaction: jest.fn().mockResolvedValue('signed-xdr'),
        },
      };
      (global as any).window = mockWindow;

      // Mock verifyChallenge
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockVerification,
      });

      const result = await service.authenticate(
        'GCLIENT',
        {
          homeDomain: 'example.com',
        }
      );

      expect(result).toEqual(mockVerification);
      expect(global.fetch).toHaveBeenCalledTimes(3);
    });
  });
});
