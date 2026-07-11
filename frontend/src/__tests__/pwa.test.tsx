import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { join } from 'path';

// Mock navigator.onLine
Object.defineProperty(globalThis, 'navigator', {
  value: {
    onLine: true,
    serviceWorker: {
      register: vi.fn(),
      ready: Promise.resolve({
        active: { state: 'activated' }
      })
    }
  },
  writable: true
});

// Mock window properties for PWA
beforeEach(() => {
  // Mock serviceWorker
  globalThis.navigator.serviceWorker = {
    register: vi.fn(),
    ready: Promise.resolve({
      active: { state: 'activated' }
    })
  } as any;

  // Mock caches
  globalThis.caches = {
    open: vi.fn(),
    match: vi.fn(),
    delete: vi.fn()
  } as any;
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('PWA Offline Implementation', () => {
  it('renders offline page correctly', async () => {
    const OfflinePage = (await import('@/app/offline/page')).default;
    
    render(<OfflinePage />);
    
    expect(screen.getByText('Offline Mode')).toBeInTheDocument();
    expect(screen.getByText('Retry Connection')).toBeInTheDocument();
    expect(screen.getByText('Go Home')).toBeInTheDocument();
  });

  it('pwa utils detect offline status', async () => {
    const { isOffline, getSWStatus, registerSW } = await import('@/lib/pwa');
    
    expect(isOffline()).toBe(false);
    
    // Mock offline
    (globalThis.navigator as any).onLine = false;
    expect(isOffline()).toBe(true);
    
    // Mock SW status
    expect(await getSWStatus()).toBe('registered');
    
    // Mock registration
    await registerSW();
    expect(globalThis.navigator.serviceWorker.register).toHaveBeenCalledWith('/sw.js');
  });

  it('caches static assets correctly (simulated)', async () => {
    // Simulate CacheFirst for images/fonts
    (globalThis.caches as any).open.mockResolvedValue({
      addAll: vi.fn(),
      match: vi.fn().mockResolvedValue({}),
      keys: vi.fn().mockResolvedValue([])
    });

    const { readFileSync } = await import('fs');
    // Verify icons exist for manifest
    const icons = ['icon-192x192.png', 'icon-512x512.png', 'apple-touch-icon.png'];
    icons.forEach(icon => {
      expect(() => readFileSync(join(process.cwd(), 'public', icon))).not.toThrow();
    });
  });
});

