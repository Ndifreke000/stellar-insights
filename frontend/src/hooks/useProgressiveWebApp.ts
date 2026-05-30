'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import { logger } from '@/lib/logger';

/**
 * PWA Installation States
 */
export enum PWAInstallState {
  UNSUPPORTED = 'unsupported',
  READY = 'ready',
  INSTALLING = 'installing',
  INSTALLED = 'installed',
  DISMISSED = 'dismissed',
}

/**
 * PWA Capabilities
 */
export interface PWACapabilities {
  canInstall: boolean;
  isInstalled: boolean;
  isOnline: boolean;
  isStandalone: boolean;
  serviceWorkerReady: boolean;
  cacheSize: number;
}

/**
 * PWA Hook Return Type
 */
export interface UseProgressiveWebAppReturn {
  state: PWAInstallState;
  capabilities: PWACapabilities;
  installPrompt: BeforeInstallPromptEvent | null;
  install: () => Promise<void>;
  dismiss: () => void;
  clearCache: () => Promise<void>;
  updateAvailable: boolean;
  updateApp: () => void;
  isLoading: boolean;
  error: Error | null;
}

/**
 * Hook for managing Progressive Web App functionality
 * Handles installation, offline detection, service worker updates, and caching
 */
export function useProgressiveWebApp(): UseProgressiveWebAppReturn {
  const [state, setState] = useState<PWAInstallState>(PWAInstallState.UNSUPPORTED);
  const [installPrompt, setInstallPrompt] = useState<BeforeInstallPromptEvent | null>(null);
  const [isOnline, setIsOnline] = useState(true);
  const [isStandalone, setIsStandalone] = useState(false);
  const [serviceWorkerReady, setServiceWorkerReady] = useState(false);
  const [cacheSize, setCacheSize] = useState(0);
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const swRegistrationRef = useRef<ServiceWorkerRegistration | null>(null);
  const dismissedRef = useRef(false);

  // Check if app is installed
  const isInstalled = useCallback(() => {
    if (typeof window === 'undefined') return false;
    return (
      window.matchMedia('(display-mode: standalone)').matches ||
      (window.navigator as any).standalone === true
    );
  }, []);

  // Initialize PWA capabilities
  useEffect(() => {
    if (typeof window === 'undefined') return;

    // Check standalone mode
    setIsStandalone(isInstalled());

    // Listen for beforeinstallprompt event
    const handleBeforeInstallPrompt = (e: Event) => {
      e.preventDefault();
      if (!dismissedRef.current) {
        setInstallPrompt(e as BeforeInstallPromptEvent);
        setState(PWAInstallState.READY);
      }
    };

    // Listen for app installed event
    const handleAppInstalled = () => {
      logger.info('PWA installed successfully');
      setState(PWAInstallState.INSTALLED);
      setInstallPrompt(null);
      setIsStandalone(true);
    };

    // Listen for online/offline events
    const handleOnline = () => {
      setIsOnline(true);
      logger.info('App is online');
    };

    const handleOffline = () => {
      setIsOnline(false);
      logger.warn('App is offline');
    };

    window.addEventListener('beforeinstallprompt', handleBeforeInstallPrompt);
    window.addEventListener('appinstalled', handleAppInstalled);
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    // Register service worker
    if ('serviceWorker' in navigator) {
      navigator.serviceWorker
        .register('/sw.js', { scope: '/' })
        .then((registration) => {
          swRegistrationRef.current = registration;
          setServiceWorkerReady(true);
          logger.info('Service Worker registered');

          // Listen for updates
          registration.addEventListener('updatefound', () => {
            const newWorker = registration.installing;
            if (newWorker) {
              newWorker.addEventListener('statechange', () => {
                if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
                  setUpdateAvailable(true);
                  logger.info('App update available');
                }
              });
            }
          });

          // Check for updates periodically
          const updateInterval = setInterval(() => {
            registration.update().catch((err) => {
              logger.error('Failed to check for SW updates:', err);
            });
          }, 60000); // Check every minute

          return () => clearInterval(updateInterval);
        })
        .catch((err) => {
          logger.error('Service Worker registration failed:', err);
          setError(err instanceof Error ? err : new Error('SW registration failed'));
        });
    }

    // Calculate cache size
    if ('storage' in navigator && 'estimate' in navigator.storage) {
      navigator.storage
        .estimate()
        .then(({ usage }) => {
          setCacheSize(usage || 0);
        })
        .catch((err) => {
          logger.warn('Failed to estimate cache size:', err);
        });
    }

    return () => {
      window.removeEventListener('beforeinstallprompt', handleBeforeInstallPrompt);
      window.removeEventListener('appinstalled', handleAppInstalled);
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, [isInstalled]);

  // Install PWA
  const install = useCallback(async () => {
    if (!installPrompt) {
      setError(new Error('Installation prompt not available'));
      return;
    }

    try {
      setIsLoading(true);
      setState(PWAInstallState.INSTALLING);

      installPrompt.prompt();
      const { outcome } = await installPrompt.userChoice;

      if (outcome === 'accepted') {
        logger.info('User accepted PWA installation');
        setState(PWAInstallState.INSTALLED);
        setInstallPrompt(null);
      } else {
        logger.info('User dismissed PWA installation');
        setState(PWAInstallState.DISMISSED);
        dismissedRef.current = true;
      }
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Installation failed');
      logger.error('PWA installation error:', error);
      setError(error);
      setState(PWAInstallState.READY);
    } finally {
      setIsLoading(false);
    }
  }, [installPrompt]);

  // Dismiss installation prompt
  const dismiss = useCallback(() => {
    setState(PWAInstallState.DISMISSED);
    dismissedRef.current = true;
    setInstallPrompt(null);
  }, []);

  // Clear cache
  const clearCache = useCallback(async () => {
    try {
      setIsLoading(true);

      // Clear all caches
      if ('caches' in window) {
        const cacheNames = await caches.keys();
        await Promise.all(cacheNames.map((name) => caches.delete(name)));
        logger.info('Cache cleared');
      }

      // Unregister service workers
      if ('serviceWorker' in navigator) {
        const registrations = await navigator.serviceWorker.getRegistrations();
        await Promise.all(registrations.map((reg) => reg.unregister()));
        logger.info('Service Workers unregistered');
      }

      setCacheSize(0);
      setServiceWorkerReady(false);
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Cache clear failed');
      logger.error('Failed to clear cache:', error);
      setError(error);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Update app
  const updateApp = useCallback(() => {
    if (!swRegistrationRef.current?.waiting) {
      logger.warn('No waiting service worker');
      return;
    }

    try {
      swRegistrationRef.current.waiting.postMessage({ type: 'SKIP_WAITING' });
      setUpdateAvailable(false);
      logger.info('App update initiated');

      // Reload after a short delay to allow SW to take over
      setTimeout(() => {
        window.location.reload();
      }, 500);
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Update failed');
      logger.error('Failed to update app:', error);
      setError(error);
    }
  }, []);

  const capabilities: PWACapabilities = {
    canInstall: state === PWAInstallState.READY,
    isInstalled: state === PWAInstallState.INSTALLED || isStandalone,
    isOnline,
    isStandalone,
    serviceWorkerReady,
    cacheSize,
  };

  return {
    state,
    capabilities,
    installPrompt,
    install,
    dismiss,
    clearCache,
    updateAvailable,
    updateApp,
    isLoading,
    error,
  };
}
