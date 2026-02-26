"use client"

import React, { Component, ErrorInfo, ReactNode } from "react"
import { AlertTriangle, RefreshCw } from "lucide-react"

interface Props {
  children: ReactNode
  fallback?: ReactNode
  onError?: (error: Error, errorInfo: ErrorInfo) => void
}

interface State {
  hasError: boolean
  error: Error | null
}

/**
 * Error boundary specifically for chart components
 * Provides a retry mechanism and non-blocking fallback UI
 */
export class ChartErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = {
      hasError: false,
      error: null,
    }
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    // Only log in development
    if (process.env.NODE_ENV === 'development') {
      console.error("ChartErrorBoundary caught an error:", error, errorInfo)
    }
    
    // Call optional error callback
    this.props.onError?.(error, errorInfo)
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null })
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback
      }

      return (
        <div className="flex flex-col items-center justify-center p-8 bg-gray-50 dark:bg-slate-900 rounded-lg border border-gray-200 dark:border-slate-700 min-h-[300px]">
          <AlertTriangle className="w-12 h-12 text-amber-500 mb-4" />
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
            Chart Error
          </h3>
          <p className="text-sm text-gray-600 dark:text-gray-400 mb-4 text-center max-w-md">
            Unable to render this chart. This might be due to invalid data or a temporary issue.
          </p>
          
          {process.env.NODE_ENV === 'development' && this.state.error && (
            <div className="mb-4 p-3 bg-gray-100 dark:bg-slate-800 rounded border border-gray-300 dark:border-slate-600 max-w-md">
              <p className="text-xs text-rose-600 dark:text-rose-400 font-mono break-all">
                {this.state.error.toString()}
              </p>
            </div>
          )}
          
          <button
            onClick={this.handleRetry}
            className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm font-medium"
          >
            <RefreshCw className="w-4 h-4" />
            Retry
          </button>
        </div>
      )
    }

    return this.props.children
  }
}
