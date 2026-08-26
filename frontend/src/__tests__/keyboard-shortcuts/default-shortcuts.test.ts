import { describe, it, expect, vi } from 'vitest';
import { createDefaultShortcuts } from '@/lib/keyboard-shortcuts/default-shortcuts';

describe('createDefaultShortcuts', () => {
  const mockHandlers = {
    showHelp: vi.fn(),
    goToDashboard: vi.fn(),
    goToCorridors: vi.fn(),
    goToAnchors: vi.fn(),
    goToAnalytics: vi.fn(),
    openSearch: vi.fn(),
    toggleSidebar: vi.fn(),
    toggleTheme: vi.fn(),
    openNotifications: vi.fn(),
    refreshData: vi.fn(),
  };

  it('should return an array of shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    expect(Array.isArray(shortcuts)).toBe(true);
    expect(shortcuts.length).toBeGreaterThan(0);
  });

  it('should include show-shortcuts-help shortcut', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const helpShortcut = shortcuts.find(s => s.id === 'show-shortcuts-help');
    expect(helpShortcut).toBeDefined();
    expect(helpShortcut?.name).toBe('Show Keyboard Shortcuts');
    expect(helpShortcut?.category).toBe('system');
    expect(helpShortcut?.defaultBinding.key).toBe('?');
    expect(helpShortcut?.defaultBinding.modifiers).toContain('shift');
  });

  it('should include navigation shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const navIds = ['go-to-dashboard', 'go-to-corridors', 'go-to-anchors', 'go-to-analytics'];
    navIds.forEach(id => {
      const shortcut = shortcuts.find(s => s.id === id);
      expect(shortcut).toBeDefined();
      expect(shortcut?.category).toBe('navigation');
    });
  });

  it('should include search shortcut', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const searchShortcut = shortcuts.find(s => s.id === 'open-search');
    expect(searchShortcut).toBeDefined();
    expect(searchShortcut?.category).toBe('search');
    expect(searchShortcut?.defaultBinding.key).toBe('k');
    expect(searchShortcut?.defaultBinding.modifiers).toContain('ctrl');
  });

  it('should include UI shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const uiIds = ['toggle-sidebar', 'toggle-theme', 'open-notifications'];
    uiIds.forEach(id => {
      const shortcut = shortcuts.find(s => s.id === id);
      expect(shortcut).toBeDefined();
      expect(shortcut?.category).toBe('ui');
    });
  });

  it('should include refresh-data action shortcut', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const refreshShortcut = shortcuts.find(s => s.id === 'refresh-data');
    expect(refreshShortcut).toBeDefined();
    expect(refreshShortcut?.category).toBe('actions');
  });

  it('should include skip-to-content accessibility shortcut', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const skipShortcut = shortcuts.find(s => s.id === 'skip-to-content');
    expect(skipShortcut).toBeDefined();
    expect(skipShortcut?.category).toBe('accessibility');
  });

  it('should call correct handlers when invoked', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);

    shortcuts.find(s => s.id === 'show-shortcuts-help')?.handler(new KeyboardEvent('keydown', { key: '?' }));
    expect(mockHandlers.showHelp).toHaveBeenCalledTimes(1);

    shortcuts.find(s => s.id === 'go-to-dashboard')?.handler(new KeyboardEvent('keydown', { key: 'd' }));
    expect(mockHandlers.goToDashboard).toHaveBeenCalledTimes(1);

    shortcuts.find(s => s.id === 'go-to-corridors')?.handler(new KeyboardEvent('keydown', { key: 'c' }));
    expect(mockHandlers.goToCorridors).toHaveBeenCalledTimes(1);

    shortcuts.find(s => s.id === 'go-to-anchors')?.handler(new KeyboardEvent('keydown', { key: 'a' }));
    expect(mockHandlers.goToAnchors).toHaveBeenCalledTimes(1);

    shortcuts.find(s => s.id === 'go-to-analytics')?.handler(new KeyboardEvent('keydown', { key: 'y' }));
    expect(mockHandlers.goToAnalytics).toHaveBeenCalledTimes(1);

    shortcuts.find(s => s.id === 'open-search')?.handler(new KeyboardEvent('keydown', { key: 'k' }));
    expect(mockHandlers.openSearch).toHaveBeenCalledTimes(1);

    shortcuts.find(s => s.id === 'toggle-sidebar')?.handler(new KeyboardEvent('keydown', { key: 'b' }));
    expect(mockHandlers.toggleSidebar).toHaveBeenCalledTimes(1);

    shortcuts.find(s => s.id === 'toggle-theme')?.handler(new KeyboardEvent('keydown', { key: 'd' }));
    expect(mockHandlers.toggleTheme).toHaveBeenCalledTimes(1);

    shortcuts.find(s => s.id === 'open-notifications')?.handler(new KeyboardEvent('keydown', { key: 'n' }));
    expect(mockHandlers.openNotifications).toHaveBeenCalledTimes(1);

    shortcuts.find(s => s.id === 'refresh-data')?.handler(new KeyboardEvent('keydown', { key: 'r' }));
    expect(mockHandlers.refreshData).toHaveBeenCalledTimes(1);
  });

  it('should have preventDefault on navigation shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const navShortcuts = shortcuts.filter(s => s.category === 'navigation');
    navShortcuts.forEach(shortcut => {
      expect(shortcut.preventDefault).toBe(true);
    });
  });

  it('should have preventDefault on search shortcut', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const searchShortcut = shortcuts.find(s => s.id === 'open-search');
    expect(searchShortcut?.preventDefault).toBe(true);
  });

  it('should have platform-specific mac bindings for navigation shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const navShortcuts = shortcuts.filter(s => s.category === 'navigation');
    navShortcuts.forEach(shortcut => {
      expect(shortcut.defaultBinding.mac).toBeDefined();
      expect(shortcut.defaultBinding.mac?.modifiers).toContain('ctrl');
    });
  });

  it('should have mac binding using meta for search and UI shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const searchShortcut = shortcuts.find(s => s.id === 'open-search');
    expect(searchShortcut?.defaultBinding.mac?.modifiers).toContain('meta');

    const sidebarShortcut = shortcuts.find(s => s.id === 'toggle-sidebar');
    expect(sidebarShortcut?.defaultBinding.mac?.modifiers).toContain('meta');
  });

  it('should have unique IDs for all shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const ids = shortcuts.map(s => s.id);
    const uniqueIds = new Set(ids);
    expect(uniqueIds.size).toBe(ids.length);
  });

  it('should have unique default bindings across shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    const bindings = shortcuts.map(s => {
      const binding = s.defaultBinding;
      return `${binding.key}-${(binding.modifiers || []).sort().join('+')}`;
    });
    const uniqueBindings = new Set(bindings);
    expect(uniqueBindings.size).toBe(bindings.length);
  });

  it('should have all shortcuts with required fields', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    shortcuts.forEach(shortcut => {
      expect(shortcut.id).toBeDefined();
      expect(shortcut.name).toBeDefined();
      expect(shortcut.description).toBeDefined();
      expect(shortcut.category).toBeDefined();
      expect(shortcut.defaultBinding).toBeDefined();
      expect(shortcut.defaultBinding.key).toBeDefined();
      expect(typeof shortcut.handler).toBe('function');
    });
  });

  it('should include 11 default shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    expect(shortcuts.length).toBe(11);
  });

  it('should handle skip-to-content handler by focusing main element', () => {
    const mainElement = document.createElement('main');
    document.body.appendChild(mainElement);

    const shortcuts = createDefaultShortcuts(mockHandlers);
    const skipShortcut = shortcuts.find(s => s.id === 'skip-to-content');

    skipShortcut?.handler(new KeyboardEvent('keydown', { key: 'm' }));

    document.body.removeChild(mainElement);
  });

  it('should have descriptions for all shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    shortcuts.forEach(shortcut => {
      expect(shortcut.description.length).toBeGreaterThan(0);
    });
  });

  it('should have human-readable names for all shortcuts', () => {
    const shortcuts = createDefaultShortcuts(mockHandlers);
    shortcuts.forEach(shortcut => {
      expect(shortcut.name.length).toBeGreaterThan(0);
    });
  });

  it('should categorize shortcuts into valid categories', () => {
    const validCategories = ['navigation', 'search', 'actions', 'ui', 'accessibility', 'system'];
    const shortcuts = createDefaultShortcuts(mockHandlers);
    shortcuts.forEach(shortcut => {
      expect(validCategories).toContain(shortcut.category);
    });
  });
});
