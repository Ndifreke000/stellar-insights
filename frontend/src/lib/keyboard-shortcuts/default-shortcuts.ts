/**
 * Default Keyboard Shortcuts
 * Application-wide keyboard shortcuts configuration
 */

import type { ShortcutAction } from '@/types/keyboard-shortcuts';

/**
 * Create default shortcuts with dynamic handlers
 * Handlers are injected at runtime to avoid circular dependencies
 */
export function createDefaultShortcuts(handlers: {
  showHelp: () => void;
  goToDashboard: () => void;
  goToCorridors: () => void;
  goToAnchors: () => void;
  goToAnalytics: () => void;
  openSearch: () => void;
  toggleSidebar: () => void;
  toggleTheme: () => void;
  openNotifications: () => void;
  refreshData: () => void;
}): ShortcutAction[] {
  return [
    // System
    {
      id: 'show-shortcuts-help',
      name: 'Show Keyboard Shortcuts',
      description: 'Display this help overlay',
      category: 'system',
      defaultBinding: {
        key: '?',
        modifiers: ['shift'],
      },
      handler: handlers.showHelp,
      preventDefault: true,
    },

    // Navigation
    {
      id: 'go-to-dashboard',
      name: 'Go to Dashboard',
      description: 'Navigate to the main dashboard',
      category: 'navigation',
      defaultBinding: {
        key: 'd',
        modifiers: ['alt'],
        mac: { key: 'd', modifiers: ['ctrl'] },
      },
      handler: handlers.goToDashboard,
      preventDefault: true,
    },
    {
      id: 'go-to-corridors',
      name: 'Go to Corridors',
      description: 'Navigate to payment corridors',
      category: 'navigation',
      defaultBinding: {
        key: 'c',
        modifiers: ['alt'],
        mac: { key: 'c', modifiers: ['ctrl'] },
      },
      handler: handlers.goToCorridors,
      preventDefault: true,
    },
    {
      id: 'go-to-anchors',
      name: 'Go to Anchors',
      description: 'Navigate to anchor directory',
      category: 'navigation',
      defaultBinding: {
        key: 'a',
        modifiers: ['alt'],
        mac: { key: 'a', modifiers: ['ctrl'] },
      },
      handler: handlers.goToAnchors,
      preventDefault: true,
    },
    {
      id: 'go-to-analytics',
      name: 'Go to Analytics',
      description: 'Navigate to analytics page',
      category: 'navigation',
      defaultBinding: {
        key: 'y',
        modifiers: ['alt'],
        mac: { key: 'y', modifiers: ['ctrl'] },
      },
      handler: handlers.goToAnalytics,
      preventDefault: true,
    },

    // Search
    {
      id: 'open-search',
      name: 'Open Search',
      description: 'Open global search',
      category: 'search',
      defaultBinding: {
        key: 'k',
        modifiers: ['ctrl'],
        mac: { key: 'k', modifiers: ['meta'] },
      },
      handler: handlers.openSearch,
      preventDefault: true,
    },

    // UI Actions
    {
      id: 'toggle-sidebar',
      name: 'Toggle Sidebar',
      description: 'Collapse or expand the sidebar',
      category: 'ui',
      defaultBinding: {
        key: 'b',
        modifiers: ['ctrl'],
        mac: { key: 'b', modifiers: ['meta'] },
      },
      handler: handlers.toggleSidebar,
      preventDefault: true,
    },
    {
      id: 'toggle-theme',
      name: 'Toggle Theme',
      description: 'Switch between light and dark theme',
      category: 'ui',
      defaultBinding: {
        key: 'd',
        modifiers: ['ctrl', 'shift'],
        mac: { key: 'd', modifiers: ['meta', 'shift'] },
      },
      handler: handlers.toggleTheme,
      preventDefault: true,
    },
    {
      id: 'open-notifications',
      name: 'Open Notifications',
      description: 'Open notification center',
      category: 'ui',
      defaultBinding: {
        key: 'n',
        modifiers: ['ctrl', 'shift'],
        mac: { key: 'n', modifiers: ['meta', 'shift'] },
      },
      handler: handlers.openNotifications,
      preventDefault: true,
    },

    // Actions
    {
      id: 'refresh-data',
      name: 'Refresh Data',
      description: 'Manually refresh current page data',
      category: 'actions',
      defaultBinding: {
        key: 'r',
        modifiers: ['ctrl', 'shift'],
        mac: { key: 'r', modifiers: ['meta', 'shift'] },
      },
      handler: handlers.refreshData,
      preventDefault: true,
    },

    // Accessibility
    {
      id: 'skip-to-content',
      name: 'Skip to Main Content',
      description: 'Skip navigation and jump to main content',
      category: 'accessibility',
      defaultBinding: {
        key: 'm',
        modifiers: ['alt'],
        mac: { key: 'm', modifiers: ['ctrl'] },
      },
      handler: () => {
        const main = document.querySelector('main');
        if (main) {
          main.focus();
          main.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
      },
      preventDefault: true,
    },
  ];
}
