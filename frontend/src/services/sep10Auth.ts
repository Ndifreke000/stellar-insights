import {
  Transaction,
  Operation,
} from '@stellar/stellar-sdk';
import { signTransaction } from '@stellar/freighter-api';
import { logger } from '@/lib/logger';

import { config } from '@/config';

const API_BASE_URL = config.apiUrl;

export interface ChallengeRequest {
  account: string;
  home_domain?: string;
  client_domain?: string;
  memo?: string;
}

export interface ChallengeResponse {
  /** Opaque challenge string to sign, exactly as received. */
  transaction: string;
  network_passphrase: string;
}

export interface VerificationRequest {
  /** The challenge transaction envelope XDR, signed by the client's wallet.
   * Signatures travel inside the envelope; SEP-10 takes no separate signature
   * field. */
  transaction: string;
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

interface FreighterApi {
  getPublicKey(): Promise<string>;
  signTransaction(
    xdr: string,
    opts: { network: string; networkPassphrase: string; accountToSign: string },
  ): Promise<string>;
}

interface AlbedoApi {
  publicKey(opts: Record<string, unknown>): Promise<{ pubkey: string }>;
  tx(opts: {
    xdr: string;
    network: string;
    pubkey: string;
  }): Promise<{ signed_envelope_xdr: string }>;
}

interface XBullSDKApi {
  signTransaction(opts: {
    xdr: string;
    network: string;
    publicKey: string;
  }): Promise<string>;
}

interface RabetApi {
  connect(): Promise<{ publicKey: string }>;
  sign(xdr: string, network: string): Promise<{ xdr: string }>;
}

declare global {
  interface Window {
    freighter?: FreighterApi;
    albedo?: AlbedoApi;
    xBullSDK?: XBullSDKApi;
    rabet?: RabetApi;
  }
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
      throw new Error("Failed to fetch SEP-10 info");
    }
    return response.json();
  }

  /**
   * Request a challenge transaction from the server
   */
  async requestChallenge(
    request: ChallengeRequest,
  ): Promise<ChallengeResponse> {
    const response = await fetch(`${this.apiBaseUrl}/api/sep10/auth`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || "Failed to request challenge");
    }

    return response.json();
  }

  /**
   * Sign the challenge transaction envelope with the account's wallet and
   * return the signed envelope XDR.
   *
   * The challenge is a server-signed Stellar transaction. The wallet adds the
   * client's signature to that envelope (preserving the server's signature),
   * and the signed envelope is posted back to the server. Signatures travel
   * inside the envelope — SEP-10 takes no separate signature field.
   */
  async signChallenge(
    challengeTransactionXdr: string,
    networkPassphrase: string,
    publicKey: string,
  ): Promise<string> {
    const signed = await signTransaction(challengeTransactionXdr, {
      networkPassphrase,
      address: publicKey,
    });

    if (signed.error || !signed.signedTxXdr) {
      logger.error("Freighter signing failed:", signed.error);
      throw new Error(
        signed.error?.message ||
          "Could not sign the login challenge. Install or unlock the Freighter wallet and try again.",
      );
    }

    return signed.signedTxXdr;
  }

  /**
   * Verify the signed challenge transaction with the server
   */
  async verifyChallenge(
    signedTransaction: string,
  ): Promise<VerificationResponse> {
    const response = await fetch(`${this.apiBaseUrl}/api/sep10/verify`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        transaction: signedTransaction,
      } satisfies VerificationRequest),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || "Failed to verify challenge");
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
    },
  ): Promise<VerificationResponse> {
    // Step 1: Get server info
    const _info = await this.getInfo();

    // Step 2: Request challenge
    const challengeRequest: ChallengeRequest = {
      account: publicKey,
      home_domain: options?.homeDomain,
      client_domain: options?.clientDomain,
      memo: options?.memo,
    };

    const challengeResponse = await this.requestChallenge(challengeRequest);

    // Step 3: Sign the challenge transaction envelope with the wallet
    const signedTransaction = await this.signChallenge(
      challengeResponse.transaction,
      challengeResponse.network_passphrase,
      publicKey,
    );

    // Step 4: Submit the signed envelope back to the server
    const verificationResponse = await this.verifyChallenge(signedTransaction);

    return verificationResponse;
  }

  /**
   * Ask the backend whether the current session is still valid.
   *
   * The session lives in an httpOnly cookie the frontend cannot read, so this
   * defers to the server. A missing or failing endpoint reports
   * `authenticated: false` rather than throwing, so a cold load never breaks.
   */
  async getSession(): Promise<{ authenticated: boolean }> {
    const response = await fetch(`${this.apiBaseUrl}/api/sep10/session`, {
      method: "GET",
      credentials: "same-origin",
      headers: { Accept: "application/json" },
    });

    if (!response.ok) {
      return { authenticated: false };
    }

    const data = (await response.json().catch(() => null)) as {
      authenticated?: boolean;
    } | null;

    return { authenticated: data?.authenticated === true };
  }

  /**
   * Logout and invalidate the current session. The backend clears the session;
   * the frontend keeps no token of its own.
   */
  async logout(): Promise<void> {
    const response = await fetch(`${this.apiBaseUrl}/api/sep10/logout`, {
      method: "POST",
      credentials: "same-origin",
    });

    if (!response.ok) {
      const error = (await response.json().catch(() => null)) as {
        error?: string;
      } | null;
      throw new Error(error?.error || "Failed to logout");
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
    clientPublicKey: string,
  ): boolean {
    try {
      const transaction = new Transaction(challengeXdr, networkPassphrase);

      // Check source account is server
      if (transaction.source !== serverPublicKey) {
        logger.error("Invalid source account");
        return false;
      }

      // Check sequence number is 0
      if (transaction.sequence !== "0") {
        logger.error("Invalid sequence number");
        return false;
      }

      // Check time bounds exist
      if (!transaction.timeBounds) {
        logger.error("Missing time bounds");
        return false;
      }

      // Check time bounds are valid
      const now = Math.floor(Date.now() / 1000);
      if (
        now < parseInt(transaction.timeBounds.minTime) ||
        now > parseInt(transaction.timeBounds.maxTime)
      ) {
        logger.error("Transaction expired or not yet valid");
        return false;
      }

      // Check first operation is ManageData
      if (transaction.operations.length === 0) {
        logger.error("No operations found");
        return false;
      }

      const firstOp = transaction.operations[0];
      if (firstOp.type !== "manageData") {
        logger.error("First operation must be ManageData");
        return false;
      }

      // Check operation source is client
      const manageDataOp = firstOp as Operation.ManageData;
      if (manageDataOp.source !== clientPublicKey) {
        logger.error("Invalid operation source");
        return false;
      }

      // Check data name contains home domain
      if (!manageDataOp.name.includes(homeDomain)) {
        logger.error("Invalid data name");
        return false;
      }

      return true;
    } catch (error) {
      logger.error("Challenge validation error:", error);
      return false;
    }
  }
}

// Export singleton instance
export const sep10AuthService = new Sep10AuthService();
