import {
  Keypair,
  Networks,
  Transaction,
  TransactionBuilder,
  Operation,
} from '@stellar/stellar-sdk';

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export interface ChallengeRequest {
  account: string;
  home_domain?: string;
  client_domain?: string;
  memo?: string;
}

export interface ChallengeResponse {
  transaction: string; // Base64-encoded XDR
  network_passphrase: string;
}

export interface VerificationRequest {
  transaction: string; // Base64-encoded signed XDR
}

export interface VerificationResponse {
  token: string;
  expires_in: number;
}

export interface Sep10Info {
  authentication_endpoint: string;
  network_passphrase: string;
  signing_key: string;
  version: string;
}

/**
 * SEP-10 Authentication Service
 * Implements Stellar Web Authentication protocol
 */
export class Sep10AuthService {
  private apiBaseUrl: string;

  constructor(apiBaseUrl: string = API_BASE_URL) {
    this.apiBaseUrl = apiBaseUrl;
  }

  /**
   * Get SEP-10 server information
   */
  async getInfo(): Promise<Sep10Info> {
    const response = await fetch(`${this.apiBaseUrl}/api/sep10/info`);
    if (!response.ok) {
      throw new Error('Failed to fetch SEP-10 info');
    }
    return response.json();
  }

  /**
   * Request a challenge transaction from the server
   */
  async requestChallenge(request: ChallengeRequest): Promise<ChallengeResponse> {
    const response = await fetch(`${this.apiBaseUrl}/api/sep10/auth`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || 'Failed to request challenge');
    }

    return response.json();
  }

  /**
   * Sign a challenge transaction using a wallet
   * This method attempts to use various Stellar wallet integrations
   */
  async signChallenge(
    challengeXdr: string,
    networkPassphrase: string,
    publicKey: string
  ): Promise<string> {
    // Try Freighter wallet first
    if (typeof window !== 'undefined' && (window as any).freighter) {
      try {
        const signedXdr = await (window as any).freighter.signTransaction(
          challengeXdr,
          {
            network: networkPassphrase,
            networkPassphrase: networkPassphrase,
            accountToSign: publicKey,
          }
        );
        return signedXdr;
      } catch (error) {
        console.error('Freighter signing failed:', error);
      }
    }

    // Try Albedo wallet
    if (typeof window !== 'undefined' && (window as any).albedo) {
      try {
        const result = await (window as any).albedo.tx({
          xdr: challengeXdr,
          network: networkPassphrase,
          pubkey: publicKey,
        });
        return result.signed_envelope_xdr;
      } catch (error) {
        console.error('Albedo signing failed:', error);
      }
    }

    // Try xBull wallet
    if (typeof window !== 'undefined' && (window as any).xBullSDK) {
      try {
        const xBullSDK = (window as any).xBullSDK;
        const result = await xBullSDK.signTransaction({
          xdr: challengeXdr,
          network: networkPassphrase,
          publicKey: publicKey,
        });
        return result;
      } catch (error) {
        console.error('xBull signing failed:', error);
      }
    }

    // Try Rabet wallet
    if (typeof window !== 'undefined' && (window as any).rabet) {
      try {
        const result = await (window as any).rabet.sign(
          challengeXdr,
          networkPassphrase
        );
        return result.xdr;
      } catch (error) {
        console.error('Rabet signing failed:', error);
      }
    }

    throw new Error(
      'No compatible Stellar wallet found. Please install Freighter, Albedo, xBull, or Rabet.'
    );
  }

  /**
   * Verify the signed challenge transaction with the server
   */
  async verifyChallenge(
    signedTransactionXdr: string
  ): Promise<VerificationResponse> {
    const response = await fetch(`${this.apiBaseUrl}/api/sep10/verify`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        transaction: signedTransactionXdr,
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || 'Failed to verify challenge');
    }

    return response.json();
  }

  /**
   * Complete SEP-10 authentication flow
   * This is the main method to authenticate a user
   */
  async authenticate(
    publicKey: string,
    options?: {
      homeDomain?: string;
      clientDomain?: string;
      memo?: string;
    }
  ): Promise<VerificationResponse> {
    // Step 1: Get server info
    const info = await this.getInfo();

    // Step 2: Request challenge
    const challengeRequest: ChallengeRequest = {
      account: publicKey,
      home_domain: options?.homeDomain,
      client_domain: options?.clientDomain,
      memo: options?.memo,
    };

    const challengeResponse = await this.requestChallenge(challengeRequest);

    // Step 3: Sign challenge with wallet
    const signedXdr = await this.signChallenge(
      challengeResponse.transaction,
      challengeResponse.network_passphrase,
      publicKey
    );

    // Step 4: Verify signed challenge
    const verificationResponse = await this.verifyChallenge(signedXdr);

    return verificationResponse;
  }

  /**
   * Logout and invalidate session
   */
  async logout(token: string): Promise<void> {
    const response = await fetch(`${this.apiBaseUrl}/api/sep10/logout`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || 'Failed to logout');
    }
  }

  /**
   * Validate a transaction is a valid SEP-10 challenge
   * This is a client-side validation before signing
   */
  validateChallengeTransaction(
    challengeXdr: string,
    serverPublicKey: string,
    networkPassphrase: string,
    homeDomain: string,
    clientPublicKey: string
  ): boolean {
    try {
      const transaction = new Transaction(challengeXdr, networkPassphrase);

      // Check source account is server
      if (transaction.source !== serverPublicKey) {
        console.error('Invalid source account');
        return false;
      }

      // Check sequence number is 0
      if (transaction.sequence !== '0') {
        console.error('Invalid sequence number');
        return false;
      }

      // Check time bounds exist
      if (!transaction.timeBounds) {
        console.error('Missing time bounds');
        return false;
      }

      // Check time bounds are valid
      const now = Math.floor(Date.now() / 1000);
      if (
        now < parseInt(transaction.timeBounds.minTime) ||
        now > parseInt(transaction.timeBounds.maxTime)
      ) {
        console.error('Transaction expired or not yet valid');
        return false;
      }

      // Check first operation is ManageData
      if (transaction.operations.length === 0) {
        console.error('No operations found');
        return false;
      }

      const firstOp = transaction.operations[0];
      if (firstOp.type !== 'manageData') {
        console.error('First operation must be ManageData');
        return false;
      }

      // Check operation source is client
      const manageDataOp = firstOp as Operation.ManageData;
      if (manageDataOp.source !== clientPublicKey) {
        console.error('Invalid operation source');
        return false;
      }

      // Check data name contains home domain
      if (!manageDataOp.name.includes(homeDomain)) {
        console.error('Invalid data name');
        return false;
      }

      return true;
    } catch (error) {
      console.error('Challenge validation error:', error);
      return false;
    }
  }
}

// Export singleton instance
export const sep10AuthService = new Sep10AuthService();
