import { describe, expect, it, vi } from 'vitest';

vi.mock('next-intl/middleware', () => ({ default: () => () => undefined }));

import { isUnlocalizedPath } from '@/proxy';

describe('isUnlocalizedPath', () => {
  it('serves pages that live outside app/[locale] directly', () => {
    for (const path of ['/offline', '/alerts', '/api-docs', '/settings/gdpr']) {
      expect(isUnlocalizedPath(path)).toBe(true);
    }
  });

  it('matches nested paths under an unlocalized page', () => {
    expect(isUnlocalizedPath('/api-docs/v1')).toBe(true);
  });

  it('leaves localized and look-alike paths to next-intl', () => {
    for (const path of ['/', '/en/dashboard', '/dashboard', '/settings', '/offline-status', '/en/offline']) {
      expect(isUnlocalizedPath(path)).toBe(false);
    }
  });
});
