'use client';

import React, { createContext, useContext, useCallback, useEffect, useState } from 'react';

export type Theme = 'dark' | 'light';
export type ThemePreference = 'dark' | 'light' | 'system';

interface ThemeContextType {
  /** The resolved active theme (always 'dark' or 'light') */
  theme: Theme;
  /** The user's preference ('dark' | 'light' | 'system') */
  themePreference: ThemePreference;
  /** Update the preference and persist to localStorage */
  setThemePreference: (preference: ThemePreference) => void;
}

const STORAGE_KEY = 'stellar-theme-preference';

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

/** Resolve the active theme from a preference + system media query */
function resolveTheme(preference: ThemePreference): Theme {
  if (preference === 'system') {
    if (typeof window !== 'undefined') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }
    return 'dark'; // SSR fallback
  }
  return preference;
}

/** Read the saved preference from localStorage (or fall back to 'system') */
function getSavedPreference(): ThemePreference {
  if (typeof window === 'undefined') return 'system';
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored === 'dark' || stored === 'light' || stored === 'system') return stored;
  } catch {
    // storage unavailable
  }
  return 'system';
}

/** Apply the resolved theme to the <html> element */
function applyThemeToDOM(theme: Theme) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.classList.remove('dark', 'light');
  root.classList.add(theme);
  root.setAttribute('data-theme', theme);
}

interface ThemeProviderProps {
  children: React.ReactNode;
}

export function ThemeProvider({ children }: ThemeProviderProps) {
  const [themePreference, setThemePreferenceState] = useState<ThemePreference>(() => getSavedPreference());
  const [theme, setTheme] = useState<Theme>(() => resolveTheme(themePreference));

  // Persist preference to localStorage and update resolved theme
  const setThemePreference = useCallback((preference: ThemePreference) => {
    setThemePreferenceState(preference);
    try {
      window.localStorage.setItem(STORAGE_KEY, preference);
    } catch {
      // storage unavailable
    }
    const resolved = resolveTheme(preference);
    setTheme(resolved);
    applyThemeToDOM(resolved);
  }, []);

  // On mount, apply the initial theme
  useEffect(() => {
    const resolved = resolveTheme(themePreference);
    setTheme(resolved);
    applyThemeToDOM(resolved);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Listen for system preference changes when mode is 'system'
  useEffect(() => {
    if (themePreference !== 'system') return;

    const mq = window.matchMedia('(prefers-color-scheme: dark)');

    const handler = (e: MediaQueryListEvent) => {
      const next: Theme = e.matches ? 'dark' : 'light';
      setTheme(next);
      applyThemeToDOM(next);
    };

    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, [themePreference]);

  // Sync across tabs via the storage event
  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key !== STORAGE_KEY || e.newValue === null) return;
      const pref = e.newValue as ThemePreference;
      if (pref === 'dark' || pref === 'light' || pref === 'system') {
        setThemePreferenceState(pref);
        const resolved = resolveTheme(pref);
        setTheme(resolved);
        applyThemeToDOM(resolved);
      }
    };

    window.addEventListener('storage', handleStorage);
    return () => window.removeEventListener('storage', handleStorage);
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, themePreference, setThemePreference }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextType {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
}
