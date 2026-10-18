import axios from 'axios'

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

/**
 * SEP-10 Web Authentication service.
 *
 * The home domain used to validate the SEP-10 challenge is server-authoritative.
 * It is fetched from the backend config endpoint and never derived from
 * `window.location.hostname`, so preview deployments and localhost work while
 * the client cannot choose the domain that gets validated.
 */

export interface Sep10AuthOptions {
  /**
   * @deprecated The home domain is resolved from the server config endpoint.
   * Any value passed here is ignored; the server's configured home domain is
   * authoritative for SEP-10 validation.
   */
  homeDomain?: string
}

export interface Sep10AuthResult {
  token: string
  expires_in: number
}

export interface HomeDomainConfig {
  home_domain: string
}

export class Sep10AuthError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'Sep10AuthError'
  }
}

class Sep10AuthService {
  private cachedHomeDomain: string | null = null
  private homeDomainPromise: Promise<string> | null = null

  /**
   * Fetch the server-configured home domain.
   *
   * Explicitly handles loading (in-flight dedupe), empty, error and
   * partial-data states. Never fabricates a fallback domain.
   */
  async getHomeDomain(): Promise<string> {
    if (this.cachedHomeDomain) {
      return this.cachedHomeDomain
    }

    if (this.homeDomainPromise) {
      return this.homeDomainPromise
    }

    this.homeDomainPromise = this.fetchHomeDomain()

    try {
      const homeDomain = await this.homeDomainPromise
      this.cachedHomeDomain = homeDomain
      return homeDomain
    } finally {
      this.homeDomainPromise = null
    }
  }

  private async fetchHomeDomain(): Promise<string> {
    let response
    try {
      response = await axios.get<HomeDomainConfig>(
        `${API_BASE_URL}/api/v1/config/home-domain`,
        { timeout: 10000 },
      )
    } catch (error) {
      throw new Sep10AuthError(
        'Unable to load the SEP-10 home domain configuration from the server.',
      )
    }

    const homeDomain = response?.data?.home_domain

    if (typeof homeDomain !== 'string' || homeDomain.trim() === '') {
      throw new Sep10AuthError(
        'The server returned an empty or invalid SEP-10 home domain configuration.',
      )
    }

    return homeDomain.trim()
  }

  /**
   * Perform SEP-10 authentication for the given Stellar address.
   *
   * The `homeDomain` option is accepted for backwards compatibility but is
   * ignored: the server's configured home domain is authoritative.
   */
  async authenticate(
    address: string,
    _options: Sep10AuthOptions = {},
  ): Promise<Sep10AuthResult> {
    if (!address || address.trim() === '') {
      throw new Sep10AuthError('A Stellar address is required for SEP-10 authentication.')
    }

    const homeDomain = await this.getHomeDomain()

    let response
    try {
      response = await axios.post<Sep10AuthResult>(
        `${API_BASE_URL}/api/v1/auth/sep10`,
        { account: address, home_domain: homeDomain },
        { timeout: 15000 },
      )
    } catch (error) {
      throw new Sep10AuthError('SEP-10 authentication failed.')
    }

    const result = response?.data

    if (
      !result ||
      typeof result.token !== 'string' ||
      result.token === '' ||
      typeof result.expires_in !== 'number' ||
      !Number.isFinite(result.expires_in) ||
      result.expires_in <= 0
    ) {
      throw new Sep10AuthError(
        'The server returned an invalid SEP-10 authentication response.',
      )
    }

    return result
  }
}

export const sep10AuthService = new Sep10AuthService()
