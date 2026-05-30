'use client';

import React, { useState, useEffect, useCallback } from 'react';
import { useProgressiveWebApp, PWAInstallState } from '@/hooks/useProgressiveWebApp';
import { Button } from '@/components/ui/button';
import { AlertCircle, Download, CheckCircle, Wifi, WifiOff, Trash2, RefreshCw } from 'lucide-react';
import { cn } from '@/lib/utils';

/**
 * PWA Component Configuration
 */
interface ProgressiveWebAppProps {
  /** Show installation prompt */
  showInstallPrompt?: boolean;
  /** Show offline indicator */
  showOfflineIndicator?: boolean;
  /** Show update notification */
  showUpdateNotification?: boolean;
  /** Show cache management */
  showCacheManagement?: boolean;
  /** Custom className */
  className?: string;
  /** Callback when installation state changes */
  onStateChange?: (state: PWAInstallState) => void;
}

/**
 * Progressive Web App Component
 * Provides installation, offline detection, updates, and cache management
 */
export const ProgressiveWebApp: React.FC<ProgressiveWebAppProps> = ({
  showInstallPrompt = true,
  showOfflineIndicator = true,
  showUpdateNotification = true,
  showCacheManagement = false,
  className,
  onStateChange,
}) => {
  const {
    state,
    capabilities,
    install,
    dismiss,
    clearCache,
    updateAvailable,
    updateApp,
    isLoading,
    error,
  } = useProgressiveWebApp();

  const [showInstallUI, setShowInstallUI] = useState(false);
  const [showUpdateUI, setShowUpdateUI] = useState(false);
  const [showCacheUI, setShowCacheUI] = useState(false);
  const [cacheFormatted, setCacheFormatted] = useState('0 B');

  // Format cache size
  useEffect(() => {
    const formatBytes = (bytes: number) => {
      if (bytes === 0) return '0 B';
      const k = 1024;
      const sizes = ['B', 'KB', 'MB', 'GB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
    };
    setCacheFormatted(formatBytes(capabilities.cacheSize));
  }, [capabilities.cacheSize]);

  // Notify state changes
  useEffect(() => {
    onStateChange?.(state);
  }, [state, onStateChange]);

  // Show/hide install UI
  useEffect(() => {
    setShowInstallUI(showInstallPrompt && capabilities.canInstall && !capabilities.isInstalled);
  }, [showInstallPrompt, capabilities.canInstall, capabilities.isInstalled]);

  // Show/hide update UI
  useEffect(() => {
    setShowUpdateUI(showUpdateNotification && updateAvailable && capabilities.isInstalled);
  }, [showUpdateNotification, updateAvailable, capabilities.isInstalled]);

  const handleInstall = useCallback(async () => {
    await install();
    setShowInstallUI(false);
  }, [install]);

  const handleDismiss = useCallback(() => {
    dismiss();
    setShowInstallUI(false);
  }, [dismiss]);

  const handleClearCache = useCallback(async () => {
    if (confirm('Are you sure you want to clear all cached data? This cannot be undone.')) {
      await clearCache();
      setShowCacheUI(false);
    }
  }, [clearCache]);

  const handleUpdateApp = useCallback(() => {
    updateApp();
  }, [updateApp]);

  return (
    <div className={cn('space-y-3', className)}>
      {/* Error Display */}
      {error && (
        <div className="flex items-start gap-3 p-4 bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-900/50 rounded-lg">
          <AlertCircle className="w-5 h-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-red-900 dark:text-red-200">Error</p>
            <p className="text-sm text-red-700 dark:text-red-300 mt-1">{error.message}</p>
          </div>
        </div>
      )}

      {/* Installation Prompt */}
      {showInstallUI && (
        <div className="flex items-start gap-3 p-4 bg-blue-50 dark:bg-blue-950/20 border border-blue-200 dark:border-blue-900/50 rounded-lg animate-in fade-in slide-in-from-top-2 duration-300">
          <Download className="w-5 h-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-blue-900 dark:text-blue-200">
              Install Stellar Insights
            </p>
            <p className="text-sm text-blue-700 dark:text-blue-300 mt-1">
              Get instant access to analytics with offline support and faster loading.
            </p>
            <div className="flex gap-2 mt-3">
              <Button
                size="sm"
                onClick={handleInstall}
                disabled={isLoading}
                className="bg-blue-600 hover:bg-blue-700 text-white"
              >
                {isLoading ? (
                  <>
                    <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                    Installing...
                  </>
                ) : (
                  <>
                    <Download className="w-4 h-4 mr-2" />
                    Install
                  </>
                )}
              </Button>
              <Button
                size="sm"
                variant="outline"
                onClick={handleDismiss}
                disabled={isLoading}
              >
                Not Now
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Update Available Notification */}
      {showUpdateUI && (
        <div className="flex items-start gap-3 p-4 bg-amber-50 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-900/50 rounded-lg animate-in fade-in slide-in-from-top-2 duration-300">
          <RefreshCw className="w-5 h-5 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-amber-900 dark:text-amber-200">
              Update Available
            </p>
            <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
              A new version of Stellar Insights is ready to install.
            </p>
            <div className="flex gap-2 mt-3">
              <Button
                size="sm"
                onClick={handleUpdateApp}
                disabled={isLoading}
                className="bg-amber-600 hover:bg-amber-700 text-white"
              >
                {isLoading ? (
                  <>
                    <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                    Updating...
                  </>
                ) : (
                  <>
                    <RefreshCw className="w-4 h-4 mr-2" />
                    Update Now
                  </>
                )}
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Offline Indicator */}
      {showOfflineIndicator && !capabilities.isOnline && (
        <div className="flex items-start gap-3 p-4 bg-gray-50 dark:bg-gray-950/20 border border-gray-200 dark:border-gray-900/50 rounded-lg">
          <WifiOff className="w-5 h-5 text-gray-600 dark:text-gray-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-gray-900 dark:text-gray-200">
              You're Offline
            </p>
            <p className="text-sm text-gray-700 dark:text-gray-300 mt-1">
              Using cached data. Changes will sync when you're back online.
            </p>
          </div>
        </div>
      )}

      {/* Online Indicator (when previously offline) */}
      {showOfflineIndicator && capabilities.isOnline && capabilities.isInstalled && (
        <div className="flex items-start gap-3 p-4 bg-green-50 dark:bg-green-950/20 border border-green-200 dark:border-green-900/50 rounded-lg animate-in fade-in duration-300">
          <Wifi className="w-5 h-5 text-green-600 dark:text-green-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-green-900 dark:text-green-200">
              Back Online
            </p>
            <p className="text-sm text-green-700 dark:text-green-300 mt-1">
              Syncing your data now.
            </p>
          </div>
        </div>
      )}

      {/* Installation Status */}
      {capabilities.isInstalled && (
        <div className="flex items-start gap-3 p-4 bg-emerald-50 dark:bg-emerald-950/20 border border-emerald-200 dark:border-emerald-900/50 rounded-lg">
          <CheckCircle className="w-5 h-5 text-emerald-600 dark:text-emerald-400 flex-shrink-0 mt-0.5" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-emerald-900 dark:text-emerald-200">
              App Installed
            </p>
            <p className="text-sm text-emerald-700 dark:text-emerald-300 mt-1">
              Stellar Insights is installed and ready to use offline.
            </p>
          </div>
        </div>
      )}

      {/* Cache Management */}
      {showCacheManagement && capabilities.isInstalled && (
        <div className="p-4 bg-slate-50 dark:bg-slate-950/20 border border-slate-200 dark:border-slate-900/50 rounded-lg">
          <div className="flex items-center justify-between mb-3">
            <div>
              <p className="text-sm font-medium text-slate-900 dark:text-slate-200">
                Cache Management
              </p>
              <p className="text-xs text-slate-600 dark:text-slate-400 mt-1">
                Storage used: {cacheFormatted}
              </p>
            </div>
            <Button
              size="sm"
              variant="outline"
              onClick={() => setShowCacheUI(!showCacheUI)}
              disabled={isLoading}
            >
              {showCacheUI ? 'Hide' : 'Show'}
            </Button>
          </div>

          {showCacheUI && (
            <div className="mt-3 pt-3 border-t border-slate-200 dark:border-slate-900/50">
              <p className="text-xs text-slate-600 dark:text-slate-400 mb-3">
                Clearing cache will remove all offline data and free up storage space.
              </p>
              <Button
                size="sm"
                variant="destructive"
                onClick={handleClearCache}
                disabled={isLoading || capabilities.cacheSize === 0}
              >
                {isLoading ? (
                  <>
                    <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                    Clearing...
                  </>
                ) : (
                  <>
                    <Trash2 className="w-4 h-4 mr-2" />
                    Clear Cache
                  </>
                )}
              </Button>
            </div>
          )}
        </div>
      )}

      {/* Service Worker Status */}
      {capabilities.isInstalled && (
        <div className="text-xs text-slate-500 dark:text-slate-400 px-4 py-2">
          {capabilities.serviceWorkerReady ? (
            <span>✓ Service Worker active</span>
          ) : (
            <span>⚠ Service Worker initializing...</span>
          )}
        </div>
      )}
    </div>
  );
};

export default ProgressiveWebApp;
