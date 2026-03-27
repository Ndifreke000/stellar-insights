/**
 * Functional tests for UserPreferencesContext and UserPreferencesProvider.
 *
 * Covers: reading defaults, setPrefs (partial merge), resetPrefs,
 * and persistence through the useLocalStorage hook.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import React from 'react';
import {
  UserPreferencesProvider,
  useUserPreferences,
  DEFAULT_PREFERENCES,
} from '@/contexts/UserPreferencesContext';

// ── setup ────────────────────────────────────────────────────────────────────

const wrapper = ({ children }: { children: React.ReactNode }) =>
  React.createElement(UserPreferencesProvider, null, children);

beforeEach(() => {
  window.localStorage.clear();
});

// ── default preferences ───────────────────────────────────────────────────────

describe('UserPreferencesContext – defaults', () => {
  it('returns DEFAULT_PREFERENCES when nothing is stored', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });
    expect(result.current.prefs).toEqual(DEFAULT_PREFERENCES);
  });

  it('default locale is "en"', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });
    expect(result.current.prefs.locale).toBe('en');
  });

  it('sidebar is not collapsed by default', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });
    expect(result.current.prefs.sidebarCollapsed).toBe(false);
  });

  it('default corridors view is grid', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });
    expect(result.current.prefs.corridorsViewMode).toBe('grid');
  });

  it('default sort is health_score', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });
    expect(result.current.prefs.corridorsSortBy).toBe('health_score');
  });

  it('default dashboard refresh is 30 000 ms', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });
    expect(result.current.prefs.dashboardRefreshInterval).toBe(30_000);
  });
});

// ── setPrefs (partial merge) ─────────────────────────────────────────────────

describe('UserPreferencesContext – setPrefs', () => {
  it('merges a single changed field without affecting others', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });

    act(() => {
      result.current.setPrefs({ sidebarCollapsed: true });
    });

    expect(result.current.prefs.sidebarCollapsed).toBe(true);
    // Other fields untouched
    expect(result.current.prefs.locale).toBe(DEFAULT_PREFERENCES.locale);
    expect(result.current.prefs.corridorsViewMode).toBe(DEFAULT_PREFERENCES.corridorsViewMode);
  });

  it('updates locale', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });

    act(() => {
      result.current.setPrefs({ locale: 'es' });
    });

    expect(result.current.prefs.locale).toBe('es');
  });

  it('updates corridorsViewMode to heatmap', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });

    act(() => {
      result.current.setPrefs({ corridorsViewMode: 'heatmap' });
    });

    expect(result.current.prefs.corridorsViewMode).toBe('heatmap');
  });

  it('updates corridorsSortBy to volume', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });

    act(() => {
      result.current.setPrefs({ corridorsSortBy: 'liquidity' });
    });

    expect(result.current.prefs.corridorsSortBy).toBe('liquidity');
  });

  it('updates dashboardRefreshInterval', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });

    act(() => {
      result.current.setPrefs({ dashboardRefreshInterval: 60_000 });
    });

    expect(result.current.prefs.dashboardRefreshInterval).toBe(60_000);
  });

  it('applies multiple changes in a single call', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });

    act(() => {
      result.current.setPrefs({ locale: 'zh', sidebarCollapsed: true, corridorsTimePeriod: '30d' });
    });

    expect(result.current.prefs.locale).toBe('zh');
    expect(result.current.prefs.sidebarCollapsed).toBe(true);
    expect(result.current.prefs.corridorsTimePeriod).toBe('30d');
  });

  it('successive setPrefs calls accumulate correctly', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });

    act(() => { result.current.setPrefs({ locale: 'es' }); });
    act(() => { result.current.setPrefs({ sidebarCollapsed: true }); });

    expect(result.current.prefs.locale).toBe('es');
    expect(result.current.prefs.sidebarCollapsed).toBe(true);
  });
});

// ── resetPrefs ────────────────────────────────────────────────────────────────

describe('UserPreferencesContext – resetPrefs', () => {
  it('resets all fields back to DEFAULT_PREFERENCES', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });

    act(() => {
      result.current.setPrefs({ locale: 'zh', sidebarCollapsed: true });
    });

    act(() => {
      result.current.resetPrefs();
    });

    expect(result.current.prefs).toEqual(DEFAULT_PREFERENCES);
  });

  it('removes the key from localStorage', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });
    act(() => { result.current.setPrefs({ locale: 'es' }); });
    act(() => { result.current.resetPrefs(); });

    // After reset, localStorage should not contain the prefs key
    expect(window.localStorage.getItem('stellar-user-prefs')).toBeNull();
  });
});

// ── error when used outside provider ─────────────────────────────────────────

describe('UserPreferencesContext – guard', () => {
  it('throws when useUserPreferences is called outside a provider', () => {
    // renderHook without a wrapper means no provider is in the tree
    expect(() => renderHook(() => useUserPreferences())).toThrow();
  });
});

// ── persistence ───────────────────────────────────────────────────────────────

describe('UserPreferencesContext – persistence', () => {
  it('persists prefs to localStorage after setPrefs', () => {
    const { result } = renderHook(() => useUserPreferences(), { wrapper });

    act(() => { result.current.setPrefs({ locale: 'zh' }); });

    const stored = window.localStorage.getItem('stellar-user-prefs');
    expect(stored).not.toBeNull();
    const parsed = JSON.parse(stored!);
    expect(parsed.locale).toBe('zh');
  });
});
