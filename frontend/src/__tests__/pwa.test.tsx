import { existsSync, readFileSync } from 'fs';
import { join } from 'path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

beforeEach(() => {
  // The shared setup defines navigator.serviceWorker as writable.
  (navigator as unknown as { serviceWorker: unknown }).serviceWorker = {
    register: vi.fn().mockResolvedValue({}),
    ready: Promise.resolve({ active: { state: 'activated' } }),
  };
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.resetModules();
});

describe('PWA offline support', () => {
  it('renders the offline page with a retry action and a way home', async () => {
    const OfflinePage = (await import('@/app/offline/page')).default;

    render(<OfflinePage />);

    expect(screen.getByText('Offline Mode')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Home' })).toHaveAttribute('href', '/');
  });

  it('tracks connectivity from the online and offline events', async () => {
    const { addOfflineListener, isOffline } = await import('@/lib/pwa');
    const onChange = vi.fn();
    addOfflineListener(onChange);

    expect(isOffline()).toBe(false);

    window.dispatchEvent(new Event('offline'));
    expect(isOffline()).toBe(true);
    expect(onChange).toHaveBeenLastCalledWith(false);

    window.dispatchEvent(new Event('online'));
    expect(isOffline()).toBe(false);
    expect(onChange).toHaveBeenLastCalledWith(true);
  });

  it('registers the service worker script', async () => {
    const { registerSW } = await import('@/lib/pwa');

    await registerSW();

    expect(navigator.serviceWorker.register).toHaveBeenCalledWith('/sw.js');
  });

  it('ships every icon the web manifest references', () => {
    const publicDir = join(process.cwd(), 'public');
    const manifest = JSON.parse(readFileSync(join(publicDir, 'manifest.json'), 'utf8')) as {
      icons: Array<{ src: string }>;
    };

    expect(manifest.icons.length).toBeGreaterThan(0);
    for (const icon of manifest.icons) {
      expect(existsSync(join(publicDir, icon.src)), `${icon.src} is missing from public/`).toBe(true);
    }
  });
});
