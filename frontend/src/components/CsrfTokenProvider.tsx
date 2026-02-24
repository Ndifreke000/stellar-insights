'use client';

/**
 * CSRF Token Provider Component
 * 
 * Injects CSRF token into the page as a meta tag for client-side access.
 * Should be included in the root layout.
 */

import { useEffect, useState } from 'react';

export function CsrfTokenProvider() {
  const [token, setToken] = useState<string | null>(null);

  useEffect(() => {
    // Extract token from cookie
    const cookies = document.cookie.split(';');
    for (const cookie of cookies) {
      const [name, value] = cookie.trim().split('=');
      if (name === 'csrf-token') {
        setToken(decodeURIComponent(value));
        break;
      }
    }
  }, []);

  if (!token) return null;

  return (
    <meta name="csrf-token" content={token} />
  );
}
