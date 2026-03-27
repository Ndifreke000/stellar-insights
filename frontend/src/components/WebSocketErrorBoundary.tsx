"use client"

import React, { useState, ReactNode } from "react"
import { ErrorBoundary } from "./ErrorBoundary"
import { WifiOff, RefreshCw, X } from "lucide-react"

interface Props {
  children: ReactNode
  onError?: (error: Error) => void
}

/**
 * Error boundary wrapper for WebSocket-driven components
 * Provides non-blocking fallback UI that can be dismissed
 */
export function WebSocketErrorBoundary({ children, onError }: Props) {
  const [isDismissed, setIsDismissed] = useState(false)
  const [errorKey, setErrorKey] = useState(0)

  const handleReset = () => {
    setIsDismissed(false)
    setErrorKey((prev) => prev + 1)
  }

  const handleDismiss = () => {
    setIsDismissed(true)
  }

  const fallback = isDismissed ? null : (
    <div className="relative p-4 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
      <button
        onClick={handleDismiss}
        className="absolute top-2 right-2 p-1 text-amber-600 dark:text-amber-400 hover:bg-amber-100 dark:hover:bg-amber-800/50 rounded transition-colors"
        aria-label="Dismiss error"
      >
        <X className="w-4 h-4" />
      </button>
      
      <div className="flex items-start gap-3 pr-8">
        <WifiOff className="w-5 h-5 text-amber-600 dark:text-amber-400 shrink-0 mt-0.5" />
        <div className="flex-1">
          <h4 className="text-sm font-semibold text-amber-900 dark:text-amber-100 mb-1">
            Connection Issue
          </h4>
          <p className="text-sm text-amber-800 dark:text-amber-200 mb-3">
            Unable to establish real-time connection. Data may not update automatically.
          </p>
          <button
            onClick={handleReset}
            className="flex items-center gap-2 px-3 py-1.5 bg-amber-600 text-white rounded hover:bg-amber-700 transition-colors text-sm font-medium"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            Retry Connection
          </button>
        </div>
      </div>
    </div>
  )

  return (
    <ErrorBoundary
      key={errorKey}
      fallback={fallback}
      onError={(error, errorInfo) => {
        if (process.env.NODE_ENV === 'development') {
          console.error("WebSocket component error:", error, errorInfo)
        }
        onError?.(error)
      }}
    >
      {children}
    </ErrorBoundary>
  )
}
