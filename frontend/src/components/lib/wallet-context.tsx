'use client'

import React from "react"

import { createContext, useContext, useState, useCallback, useEffect } from 'react'
import { sep10AuthService } from '../../services/sep10Auth'

// Extend Window interface for Stellar wallets
declare global {
  interface Window {
    stellar?: {
      requestPublicKey: () => Promise<string>
    }
    freighter?: any
    albedo?: any
    xBullSDK?: any
    rabet?: any
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
  AUTH_TOKEN: 'stellar_sep10_token',
  TOKEN_EXPIRY: 'stellar_sep10_token_expiry',
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
        const savedAddress = localStorage.getItem(STORAGE_KEYS.ADDRESS)
        const savedToken = localStorage.getItem(STORAGE_KEYS.AUTH_TOKEN)
        const tokenExpiry = localStorage.getItem(STORAGE_KEYS.TOKEN_EXPIRY)

        if (savedAddress) {
          setAddress(savedAddress)
          setIsConnected(true)
        }

        // Check if token is still valid
        if (savedToken && tokenExpiry) {
          const expiryTime = parseInt(tokenExpiry, 10)
          const now = Date.now()

          if (now < expiryTime) {
            setAuthToken(savedToken)
            setIsAuthenticated(true)
          } else {
            // Token expired, clear it
            localStorage.removeItem(STORAGE_KEYS.AUTH_TOKEN)
            localStorage.removeItem(STORAGE_KEYS.TOKEN_EXPIRY)
          }
        }
      } catch (error) {
        console.error('Error checking wallet connection:', error)
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
          console.error('Freighter connection failed:', error)
        }
      }

      // Try generic stellar interface
      if (!publicKey && typeof window !== 'undefined' && window.stellar) {
        try {
          publicKey = await window.stellar.requestPublicKey()
        } catch (error) {
          console.error('Stellar wallet connection failed:', error)
        }
      }

      // Try Albedo wallet
      if (!publicKey && typeof window !== 'undefined' && window.albedo) {
        try {
          const result = await window.albedo.publicKey({})
          publicKey = result.pubkey
        } catch (error) {
          console.error('Albedo connection failed:', error)
        }
      }

      // Try Rabet wallet
      if (!publicKey && typeof window !== 'undefined' && window.rabet) {
        try {
          const result = await window.rabet.connect()
          publicKey = result.publicKey
        } catch (error) {
          console.error('Rabet connection failed:', error)
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
      console.error('Error connecting wallet:', error)
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
      // Perform SEP-10 authentication
      const result = await sep10AuthService.authenticate(address, {
        homeDomain: window.location.hostname,
      })

      // Store token and expiry
      const expiryTime = Date.now() + result.expires_in * 1000
      localStorage.setItem(STORAGE_KEYS.AUTH_TOKEN, result.token)
      localStorage.setItem(STORAGE_KEYS.TOKEN_EXPIRY, expiryTime.toString())

      setAuthToken(result.token)
      setIsAuthenticated(true)
    } catch (error) {
      console.error('SEP-10 authentication failed:', error)
      throw error
    }
  }, [address])

  const logout = useCallback(async () => {
    if (authToken) {
      try {
        await sep10AuthService.logout(authToken)
      } catch (error) {
        console.error('Logout failed:', error)
      }
    }

    // Clear authentication state
    setAuthToken(null)
    setIsAuthenticated(false)
    localStorage.removeItem(STORAGE_KEYS.AUTH_TOKEN)
    localStorage.removeItem(STORAGE_KEYS.TOKEN_EXPIRY)
  }, [authToken])

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
