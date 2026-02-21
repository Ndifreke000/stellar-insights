'use client'

import { useWallet } from '../components/lib/wallet-context'
import { LogOut, Wallet, Shield, CheckCircle } from 'lucide-react'
import { useState } from 'react'

export function WalletButton() {
  const {
    isConnected,
    address,
    isConnecting,
    isAuthenticated,
    connectWallet,
    disconnectWallet,
    authenticateWithSep10,
    logout,
  } = useWallet()
  const [showMenu, setShowMenu] = useState(false)
  const [isAuthenticating, setIsAuthenticating] = useState(false)

  const handleConnect = async () => {
    try {
      await connectWallet()
    } catch (error) {
      console.error('Failed to connect wallet:', error)
    }
  }

  const handleAuthenticate = async () => {
    setIsAuthenticating(true)
    try {
      await authenticateWithSep10()
      setShowMenu(false)
    } catch (error) {
      console.error('Failed to authenticate:', error)
      alert('Authentication failed. Please try again.')
    } finally {
      setIsAuthenticating(false)
    }
  }

  const handleLogout = async () => {
    try {
      await logout()
      setShowMenu(false)
    } catch (error) {
      console.error('Failed to logout:', error)
    }
  }

  if (isConnected && address) {
    return (
      <div className="relative">
        <button
          onClick={() => setShowMenu(!showMenu)}
          className={`px-4 py-2 rounded-full font-medium hover:opacity-90 transition flex items-center gap-2 ${
            isAuthenticated
              ? 'bg-green-500 text-white'
              : 'bg-blue-500 text-white'
          }`}
        >
          {isAuthenticated ? (
            <CheckCircle className="w-4 h-4" />
          ) : (
            <Wallet className="w-4 h-4" />
          )}
          <span className="hidden sm:inline">
            {address.slice(0, 6)}...{address.slice(-4)}
          </span>
          <span className="sm:hidden">
            {isAuthenticated ? 'Authenticated' : 'Wallet'}
          </span>
        </button>

        {showMenu && (
          <div className="absolute right-0 mt-2 w-64 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg py-2 z-50">
            <div className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 border-b border-gray-200 dark:border-gray-700">
              <div className="font-mono text-xs break-all">{address}</div>
              {isAuthenticated && (
                <div className="flex items-center gap-1 mt-1 text-green-600 dark:text-green-400">
                  <CheckCircle className="w-3 h-3" />
                  <span className="text-xs">Authenticated</span>
                </div>
              )}
            </div>

            {!isAuthenticated && (
              <button
                onClick={handleAuthenticate}
                disabled={isAuthenticating}
                className="w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2 transition disabled:opacity-50"
              >
                <Shield className="w-4 h-4" />
                {isAuthenticating ? 'Authenticating...' : 'Authenticate (SEP-10)'}
              </button>
            )}

            {isAuthenticated && (
              <button
                onClick={handleLogout}
                className="w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2 transition"
              >
                <LogOut className="w-4 h-4" />
                Logout
              </button>
            )}

            <button
              onClick={() => {
                disconnectWallet()
                setShowMenu(false)
              }}
              className="w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700 flex items-center gap-2 transition border-t border-gray-200 dark:border-gray-700"
            >
              <LogOut className="w-4 h-4" />
              Disconnect Wallet
            </button>
          </div>
        )}
      </div>
    )
  }

  return (
    <button
      onClick={handleConnect}
      disabled={isConnecting}
      className="px-6 py-2 bg-blue-500 text-white rounded-full font-medium hover:opacity-90 transition disabled:opacity-50 flex items-center gap-2"
    >
      <Wallet className="w-4 h-4" />
      {isConnecting ? 'Connecting...' : 'Connect Wallet'}
    </button>
  )
}
