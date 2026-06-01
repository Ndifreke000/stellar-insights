/**
 * useShortcut Hook
 * Convenient hook for registering shortcuts in components
 */

import { useEffect } from 'react';
import { useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import type { ShortcutAction } from '@/types/keyboard-shortcuts';

/**
 * Register a keyboard shortcut for the lifetime of the component
 * 
 * @example
 * ```tsx
 * useShortcut({
 *   id: 'open-search',
 *   name: 'Open Search',
 *   description: 'Open the search dialog',
 *   category: 'search',
 *   defaultBinding: { key: 'k', modifiers: ['ctrl'] },
 *   handler: () => setSearchOpen(true),
 * });
 * ```
 */
export function useShortcut(action: ShortcutAction) {
  const { registerShortcut, unregisterShortcut } = useKeyboardShortcuts();

  useEffect(() => {
    registerShortcut(action);
    return () => unregisterShortcut(action.id);
  }, [action, registerShortcut, unregisterShortcut]);
}

/**
 * Register multiple keyboard shortcuts
 */
export function useShortcuts(actions: ShortcutAction[]) {
  const { registerShortcut, unregisterShortcut } = useKeyboardShortcuts();

  useEffect(() => {
    actions.forEach(action => registerShortcut(action));
    return () => actions.forEach(action => unregisterShortcut(action.id));
  }, [actions, registerShortcut, unregisterShortcut]);
}
