/**
 * useShortcut Hook
 * Convenient hook for registering shortcuts in components
 */

import { useEffect, useRef } from 'react';
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
/**
 * Identity of a shortcut for registration purposes: everything except the
 * handler. Callers usually pass a fresh object literal on every render, so
 * depending on the object itself would re-register on every render.
 */
function registrationKey(action: ShortcutAction): string {
  const { handler: _handler, ...rest } = action;
  return JSON.stringify(rest);
}

export function useShortcut(action: ShortcutAction) {
  const { registerShortcut, unregisterShortcut } = useKeyboardShortcuts();
  const latest = useRef(action);
  const key = registrationKey(action);

  useEffect(() => {
    latest.current = action;
  });

  useEffect(() => {
    const { id } = latest.current;
    // Register a stable wrapper so the newest handler is always the one called.
    registerShortcut({ ...latest.current, handler: event => latest.current.handler(event) });
    return () => unregisterShortcut(id);
  }, [key, registerShortcut, unregisterShortcut]);
}

/**
 * Register multiple keyboard shortcuts
 */
export function useShortcuts(actions: ShortcutAction[]) {
  const { registerShortcut, unregisterShortcut } = useKeyboardShortcuts();
  const latest = useRef(actions);
  const key = actions.map(registrationKey).join('|');

  useEffect(() => {
    latest.current = actions;
  });

  useEffect(() => {
    const ids = latest.current.map(action => action.id);
    latest.current.forEach((action, index) =>
      registerShortcut({
        ...action,
        handler: event => latest.current[index]?.handler(event),
      }),
    );
    return () => ids.forEach(id => unregisterShortcut(id));
  }, [key, registerShortcut, unregisterShortcut]);
}
