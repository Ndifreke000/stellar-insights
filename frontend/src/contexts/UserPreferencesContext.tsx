"use client";

import React, { createContext, useContext, useCallback, useMemo } from "react";
import { useLocalStorage } from "@/hooks/useLocalStorage";

// ─── Preference Types ────────────────────────────────────────────────────────

export type CorridorsSortBy = "health_score" | "success_rate" | "liquidity";
export type CorridorsViewMode = "grid" | "heatmap";
export type CorridorsTimePeriod = "7d" | "30d" | "90d" | "";

export interface UserPreferences {
  /** Whether the sidebar is collapsed to icon-only mode */
  sidebarCollapsed: boolean;
  /** Corridors page: card/heatmap view */
  corridorsViewMode: CorridorsViewMode;
  /** Corridors page: default sort column */
  corridorsSortBy: CorridorsSortBy;
  /** Corridors page: default time period filter */
  corridorsTimePeriod: CorridorsTimePeriod;
  /** Dashboard: auto-refresh interval in milliseconds */
  dashboardRefreshInterval: number;
}

export const DEFAULT_PREFERENCES: UserPreferences = {
  sidebarCollapsed: false,
  corridorsViewMode: "grid",
  corridorsSortBy: "health_score",
  corridorsTimePeriod: "7d",
  dashboardRefreshInterval: 30_000,
};

// ─── Context Shape ───────────────────────────────────────────────────────────

interface UserPreferencesContextType {
  prefs: UserPreferences;
  /** Merge-update: only supply the keys you want to change */
  setPrefs: (partial: Partial<UserPreferences>) => void;
  /** Reset everything back to factory defaults */
  resetPrefs: () => void;
}

const UserPreferencesContext = createContext<
  UserPreferencesContextType | undefined
>(undefined);

// ─── Storage key ─────────────────────────────────────────────────────────────

const STORAGE_KEY = "stellar-user-prefs";

// ─── Provider ────────────────────────────────────────────────────────────────

export function UserPreferencesProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const [storedPrefs, setStoredPrefs, removeStoredPrefs] =
    useLocalStorage<UserPreferences>(STORAGE_KEY, DEFAULT_PREFERENCES);

  // Merge stored prefs with defaults so new preference keys added in future
  // releases are automatically initialised without clearing existing prefs.
  const prefs: UserPreferences = useMemo(
    () => ({ ...DEFAULT_PREFERENCES, ...storedPrefs }),
    [storedPrefs],
  );

  const setPrefs = useCallback(
    (partial: Partial<UserPreferences>) => {
      setStoredPrefs((prev) => ({
        ...DEFAULT_PREFERENCES,
        ...prev,
        ...partial,
      }));
    },
    [setStoredPrefs],
  );

  const resetPrefs = useCallback(() => {
    removeStoredPrefs();
  }, [removeStoredPrefs]);

  return (
    <UserPreferencesContext.Provider value={{ prefs, setPrefs, resetPrefs }}>
      {children}
    </UserPreferencesContext.Provider>
  );
}

// ─── Hook ────────────────────────────────────────────────────────────────────

export function useUserPreferences(): UserPreferencesContextType {
  const ctx = useContext(UserPreferencesContext);
  if (!ctx) {
    throw new Error(
      "useUserPreferences must be used within a UserPreferencesProvider",
    );
  }
  return ctx;
}
