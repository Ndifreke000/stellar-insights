'use client';

/**
 * CSRF Token Provider Component
 *
 * Fetches a fresh CSRF token on mount and keeps the meta tag in sync when
 * the API client refreshes the token after session expiry (401).
 */

import { useEffect, useState } from 'react';
import {
  CSRF_TOKEN_REFRESHED_EVENT,
  getCsrfTokenFromCookie,
  refreshCsrfToken,
} from '@/lib/csrf-client';

export function CsrfTokenProvider() {
  const [token, setToken] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    refreshCsrfToken()
      .then((freshToken) => {
        if (mounted) setToken(freshToken);
      })
      .catch(() => {
        const fallback = getCsrfTokenFromCookie();
        if (mounted && fallback) setToken(fallback);
      });

    const handleTokenRefresh = (event: Event) => {
      const detail = (event as CustomEvent<{ token: string }>).detail;
      if (detail?.token) {
        setToken(detail.token);
      }
    };

    window.addEventListener(CSRF_TOKEN_REFRESHED_EVENT, handleTokenRefresh);
    return () => {
      mounted = false;
      window.removeEventListener(CSRF_TOKEN_REFRESHED_EVENT, handleTokenRefresh);
    };
  }, []);

  if (!token) return null;

  return <meta name="csrf-token" content={token} />;
}
