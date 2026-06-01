'use client';

import { useEffect } from 'react';
import { useRouter, usePathname } from 'next/navigation';
import { useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import { useUserPreferences } from '@/contexts/UserPreferencesContext';
import { useTheme } from '@/contexts/ThemeContext';
import { createDefaultShortcuts } from '@/lib/keyboard-shortcuts/default-shortcuts';

/**
 * Initialize default keyboard shortcuts
 * This component should be mounted once in the app layout
 */
export function ShortcutsInitializer() {
  const router = useRouter();
  const pathname = usePathname();
  const { registerShortcut, showHelp } = useKeyboardShortcuts();
  const { prefs, setPrefs } = useUserPreferences();
  const { theme, setThemePreference } = useTheme();

  useEffect(() => {
    // Extract locale from pathname (e.g., /en/dashboard -> en)
    const locale = pathname.split('/')[1] || 'en';

    const shortcuts = createDefaultShortcuts({
      showHelp: () => {
        showHelp();
      },
      goToDashboard: () => {
        router.push(`/${locale}/dashboard`);
      },
      goToCorridors: () => {
        router.push(`/${locale}/corridors`);
      },
      goToAnchors: () => {
        router.push(`/${locale}/anchors`);
      },
      goToAnalytics: () => {
        router.push(`/${locale}/analytics`);
      },
      openSearch: () => {
        // Trigger search - you can implement a search modal/dialog
        const searchInput = document.querySelector<HTMLInputElement>('[data-search-input]');
        if (searchInput) {
          searchInput.focus();
        } else {
          console.log('Search functionality not yet implemented');
        }
      },
      toggleSidebar: () => {
        setPrefs({ sidebarCollapsed: !prefs.sidebarCollapsed });
      },
      toggleTheme: () => {
        // Cycle through: dark -> light -> system
        const nextTheme = theme === 'dark' ? 'light' : theme === 'light' ? 'system' : 'dark';
        setThemePreference(nextTheme);
      },
      openNotifications: () => {
        // Trigger notification center
        const notificationButton = document.querySelector<HTMLButtonElement>('[data-notification-button]');
        if (notificationButton) {
          notificationButton.click();
        }
      },
      refreshData: () => {
        // Trigger page refresh or data refetch
        window.location.reload();
      },
    });

    // Register all shortcuts
    shortcuts.forEach(shortcut => registerShortcut(shortcut));
  }, [registerShortcut, showHelp, router, pathname, prefs, setPrefs, theme, setThemePreference]);

  return null;
}
