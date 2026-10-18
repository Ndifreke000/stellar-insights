'use client'

import React from "react"

import { createContext, useContext, useState, useCallback, useEffect } from 'react'
import { sep10AuthService } from '../../services/sep10Auth'
import { logger } from "@/lib/logger"

// Extend Window interface for the generic `stellar` wallet bridge.
// freighter/albedo/xBullSDK/rabet are typed globally in services/sep10Auth.ts —
// declaring them again here with different types would conflict.
declare global {
  interface Window {
    stellar?: {
      requestPublicKey: () => Promise<string>
    }
  }
}

interface WalletContextType {
  isConnected: boolean
  address: string | null
  isConnecting: boolean
  isAuthenticated: boolean
  authToken: string | null
  connectWallet: () => Promise<void>
  disconnectWallet: () => void
  authenticateWithSep10: () => Promise<void>
  logout: () => Promise<void>
}

const WalletContext = createContext<WalletContextType | undefined>(undefined)

const STORAGE_KEYS = {
  ADDRESS: 'stellar_wallet_address',
  // Legacy keys — the auth token now lives in an httpOnly cookie set by the
  // backend after SEP-10. These are only referenced to purge old values.
  LEGACY_AUTH_TOKEN: 'stellar_sep10_token',
  LEGACY_TOKEN_EXPIRY: 'stellar_sep10_token_expiry',
}

// The SEP-10 home domain is server-authoritative. We fetch it from the config
// endpoint rather than trusting `window.location.hostname`, which breaks on
// preview deployments and localhost and lets the client pick the domain.
const CONFIG_ENDPOINT = '/api/v1/config'

async function fetchHomeDomain(): Promise<string> {
  const response = await fetch(CONFIG_ENDPOINT, {
    method: 'GET',
    headers: { Accept: 'application/json' },
    credentials: 'same-origin',
  })

  if (!response.ok) {
    throw new Error(`Failed to load config (status ${response.status})`)
  }

  const data: unknown = await response.json()
  const homeDomain =
    data && typeof data === 'object'
      ? (data as { home_domain?: unknown }).home_domain
      : undefined

  if (typeof homeDomain !== 'string' || homeDomain.trim() === '') {
    throw new Error('Config endpoint did not return a valid home_domain')
  }

  return homeDomain.trim()
}

export function WalletProvider({ children }: { children: React.ReactNode }) {
  const [isConnected, setIsConnected] = useState(false)
  const [address, setAddress] = useState<string | null>(null)
  const [isConnecting, setIsConnecting] = useState(false)
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const [authToken, setAuthToken] = useState<string | null>(null)

  // Check if wallet is already connected and authenticated on mount
  useEffect(() => {
    const checkWalletConnection = async () => {
      try {
        // Migration: purge any legacy token values left in localStorage so an
        // XSS can no longer read them. The token is now held in an httpOnly
        // cookie that JS cannot access.
        localStorage.removeItem(STORAGE_KEYS.LEGACY_AUTH_TOKEN)
        localStorage.removeItem(STORAGE_KEYS.LEGACY_TOKEN_EXPIRY)

        const savedAddress = localStorage.getItem(STORAGE_KEYS.ADDRESS)

        if (savedAddress) {
          setAddress(savedAddress)
          setIsConnected(true)
        }

        // Ask the backend whether the httpOnly session cookie is still valid.
        const session = await sep10AuthService.getSession()
        if (session?.authenticated) {
          setIsAuthenticated(true)
        }
      } catch (error) {
        logger.error('Error checking wallet connection:', error as string)
      }
    }

    checkWalletConnection()
  }, [])

  const connectWallet = useCallback(async () => {
    setIsConnecting(true)
    try {
      let publicKey: string | null = null

      // Try Freighter wallet
      if (typeof window !== 'undefined' && window.freighter) {
        try {
          publicKey = await window.freighter.getPublicKey()
        } catch (error) {
          logger.error('Freighter connection failed:', error)
        }
      }

      // Try generic stellar interface
      if (!publicKey && typeof window !== 'undefined' && window.stellar) {
        try {
          publicKey = await window.stellar.requestPublicKey()
        } catch (error) {
          logger.error('Stellar wallet connection failed:', error)
        }
      }

      // Try Albedo wallet
      if (!publicKey && typeof window !== 'undefined' && window.albedo) {
        try {
          const result = await window.albedo.publicKey({})
          publicKey = result.pubkey
        } catch (error) {
          logger.error('Albedo connection failed:', error)
        }
      }

      // Try Rabet wallet
      if (!publicKey && typeof window !== 'undefined' && window.rabet) {
        try {
          const result = await window.rabet.connect()
          publicKey = result.publicKey
        } catch (error) {
          logger.error('Rabet connection failed:', error)
        }
      }

      if (publicKey) {
        setAddress(publicKey)
        setIsConnected(true)
        localStorage.setItem(STORAGE_KEYS.ADDRESS, publicKey)
      } else {
        throw new Error(
          'No compatible Stellar wallet found. Please install Freighter, Albedo, xBull, or Rabet.'
        )
      }
    } catch (error) {
      logger.error('Error connecting wallet:', error)
      setIsConnecting(false)
      throw error
    } finally {
      setIsConnecting(false)
    }
  }, [])

  const authenticateWithSep10 = useCallback(async () => {
    if (!address) {
      throw new Error('Wallet not connected')
    }

    try {
      // Resolve the server-configured home domain. If the config endpoint is
      // unavailable or returns no domain, fail closed rather than falling back
      // to a client-controlled value.
      const homeDomain = await fetchHomeDomain()

      // Perform SEP-10 authentication. The backend sets an httpOnly session
      // cookie on success; no token is returned to or stored by the frontend.
      await sep10AuthService.authenticate(address, { homeDomain })

      setAuthToken(null)
      setIsAuthenticated(true)
    } catch (error) {
      logger.error('SEP-10 authentication failed:', error)
      throw error
    }
  }, [address])

  const logout = useCallback(async () => {
    try {
      // Backend clears the httpOnly cookie.
      await sep10AuthService.logout()
    } catch (error) {
      logger.error('Logout failed:', error)
    }

    // Clear authentication state
    setAuthToken(null)
    setIsAuthenticated(false)
    localStorage.removeItem(STORAGE_KEYS.LEGACY_AUTH_TOKEN)
    localStorage.removeItem(STORAGE_KEYS.LEGACY_TOKEN_EXPIRY)
  }, [])

  const disconnectWallet = useCallback(async () => {
    // Logout first if authenticated
    if (isAuthenticated) {
      await logout()
    }

    // Clear wallet connection
    setAddress(null)
    setIsConnected(false)
    localStorage.removeItem(STORAGE_KEYS.ADDRESS)
  }, [isAuthenticated, logout])

  return (
    <WalletContext.Provider
      value={{
        isConnected,
        address,
        isConnecting,
        isAuthenticated,
        authToken,
        connectWallet,
        disconnectWallet,
        authenticateWithSep10,
        logout,
      }}
    >
      {children}
    </WalletContext.Provider>
  )
}

export function useWallet() {
  const context = useContext(WalletContext)
  if (!context) {
    throw new Error('useWallet must be used within a WalletProvider')
  }
  return context
}
