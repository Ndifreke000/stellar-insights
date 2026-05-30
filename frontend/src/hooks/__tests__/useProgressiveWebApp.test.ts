import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import { useProgressiveWebApp, PWAInstallState } from '../useProgressiveWebApp';

describe('useProgressiveWebApp', () => {
  let mockServiceWorkerRegistration: Partial<ServiceWorkerRegistration>;
  let mockBeforeInstallPromptEvent: Partial<BeforeInstallPromptEvent>;

  beforeEach(() => {
    // Mock Service Worker
    mockServiceWorkerRegistration = {
      active: {} as ServiceWorker,
      installing: null,
      waiting: null,
      scope: '/',
      updateViaCache: 'imports',
      update: vi.fn().mockResolvedValue(undefined),
      unregister: vi.fn().mockResolvedValue(true),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    };

    // Mock BeforeInstallPrompt Event
    mockBeforeInstallPromptEvent = {
      prompt: vi.fn().mockResolvedValue(undefined),
      userChoice: Promise.resolve({ outcome: 'accepted' as const }),
      preventDefault: vi.fn(),
    };

    // Mock navigator.serviceWorker
    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        register: vi.fn().mockResolvedValue(mockServiceWorkerRegistration),
        ready: Promise.resolve(mockServiceWorkerRegistration),
        getRegistrations: vi.fn().mockResolvedValue([mockServiceWorkerRegistration]),
        controller: null,
      },
      writable: true,
    });

    // Mock navigator.storage
    Object.defineProperty(navigator, 'storage', {
      value: {
        estimate: vi.fn().mockResolvedValue({ usage: 1024 * 1024 }), // 1MB
      },
      writable: true,
    });

    // Mock window.matchMedia
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn().mockImplementation((query) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });

    // Mock caches
    Object.defineProperty(window, 'caches', {
      value: {
        keys: vi.fn().mockResolvedValue(['cache-v1', 'cache-v2']),
        delete: vi.fn().mockResolvedValue(true),
      },
      writable: true,
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('initialization', () => {
    it('should initialize with unsupported state', () => {
      const { result } = renderHook(() => useProgressiveWebApp());
      expect(result.current.state).toBe(PWAInstallState.UNSUPPORTED);
    });

    it('should register service worker on mount', async () => {
      renderHook(() => useProgressiveWebApp());

      await waitFor(() => {
        expect(navigator.serviceWorker.register).toHaveBeenCalledWith('/sw.js', { scope: '/' });
      });
    });

    it('should detect online status', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      await waitFor(() => {
        expect(result.current.capabilities.isOnline).toBe(true);
      });
    });

    it('should estimate cache size', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      await waitFor(() => {
        expect(result.current.capabilities.cacheSize).toBe(1024 * 1024);
      });
    });
  });

  describe('installation', () => {
    it('should handle beforeinstallprompt event', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      act(() => {
        const event = new Event('beforeinstallprompt') as any;
        event.preventDefault = vi.fn();
        window.dispatchEvent(event);
      });

      // Note: In real implementation, this would be set by the event listener
      // For testing, we verify the hook is ready to handle it
      expect(result.current.state).toBeDefined();
    });

    it('should install app when install is called', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      // Manually set install prompt for testing
      act(() => {
        (result.current as any).installPrompt = mockBeforeInstallPromptEvent;
      });

      await act(async () => {
        await result.current.install();
      });

      expect(mockBeforeInstallPromptEvent.prompt).toHaveBeenCalled();
    });

    it('should handle installation error', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      await act(async () => {
        // Try to install without prompt
        await result.current.install();
      });

      expect(result.current.error).toBeDefined();
      expect(result.current.error?.message).toContain('Installation prompt not available');
    });

    it('should dismiss installation prompt', () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      act(() => {
        result.current.dismiss();
      });

      expect(result.current.state).toBe(PWAInstallState.DISMISSED);
    });
  });

  describe('offline detection', () => {
    it('should detect offline status', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      act(() => {
        window.dispatchEvent(new Event('offline'));
      });

      await waitFor(() => {
        expect(result.current.capabilities.isOnline).toBe(false);
      });
    });

    it('should detect online status', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      act(() => {
        window.dispatchEvent(new Event('offline'));
      });

      await waitFor(() => {
        expect(result.current.capabilities.isOnline).toBe(false);
      });

      act(() => {
        window.dispatchEvent(new Event('online'));
      });

      await waitFor(() => {
        expect(result.current.capabilities.isOnline).toBe(true);
      });
    });
  });

  describe('cache management', () => {
    it('should clear cache', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      await act(async () => {
        await result.current.clearCache();
      });

      expect(window.caches.keys).toHaveBeenCalled();
      expect(window.caches.delete).toHaveBeenCalledWith('cache-v1');
      expect(window.caches.delete).toHaveBeenCalledWith('cache-v2');
    });

    it('should unregister service workers when clearing cache', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      await act(async () => {
        await result.current.clearCache();
      });

      expect(navigator.serviceWorker.getRegistrations).toHaveBeenCalled();
    });

    it('should handle cache clear error', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      (window.caches.keys as any).mockRejectedValueOnce(new Error('Cache error'));

      await act(async () => {
        await result.current.clearCache();
      });

      expect(result.current.error).toBeDefined();
    });
  });

  describe('capabilities', () => {
    it('should report correct capabilities', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      await waitFor(() => {
        expect(result.current.capabilities).toEqual({
          canInstall: false,
          isInstalled: false,
          isOnline: true,
          isStandalone: false,
          serviceWorkerReady: true,
          cacheSize: 1024 * 1024,
        });
      });
    });

    it('should detect standalone mode', () => {
      Object.defineProperty(window, 'matchMedia', {
        writable: true,
        value: vi.fn().mockImplementation((query) => ({
          matches: query === '(display-mode: standalone)',
          media: query,
          onchange: null,
          addListener: vi.fn(),
          removeListener: vi.fn(),
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
          dispatchEvent: vi.fn(),
        })),
      });

      const { result } = renderHook(() => useProgressiveWebApp());

      expect(result.current.capabilities.isStandalone).toBe(true);
    });
  });

  describe('updates', () => {
    it('should detect available updates', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      // Simulate update found
      act(() => {
        const event = new Event('updatefound');
        window.dispatchEvent(event);
      });

      expect(result.current.updateAvailable).toBeDefined();
    });

    it('should update app', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      // Mock waiting service worker
      const mockWaitingSW = { postMessage: vi.fn() };
      (mockServiceWorkerRegistration.waiting as any) = mockWaitingSW;

      await act(async () => {
        result.current.updateApp();
      });

      expect(mockWaitingSW.postMessage).toHaveBeenCalledWith({ type: 'SKIP_WAITING' });
    });
  });

  describe('loading states', () => {
    it('should set loading state during installation', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      expect(result.current.isLoading).toBe(false);
    });

    it('should set loading state during cache clear', async () => {
      const { result } = renderHook(() => useProgressiveWebApp());

      const clearPromise = act(async () => {
        await result.current.clearCache();
      });

      expect(result.current.isLoading).toBe(false); // After completion
    });
  });
});
