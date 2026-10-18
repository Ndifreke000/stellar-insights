'use client';

import React, { createContext, useContext, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ShortcutAction, ShortcutConfig, KeyBinding, Platform } from '@/types/keyboard-shortcuts';
import { logger } from '@/lib/logger';
import { ShortcutRegistry } from '@/lib/keyboard-shortcuts/registry';
import { detectPlatform, matchesBinding, isInputFocused } from '@/lib/keyboard-shortcuts/utils';
import { useLocalStorage } from '@/hooks/useLocalStorage';

const DEFAULT_CONFIG: ShortcutConfig = {
  customBindings: {},
  disabledShortcuts: [],
  enabled: true,
};

const STORAGE_KEY = 'stellar-keyboard-shortcuts';

/** Everything about a shortcut that a consumer can render, i.e. not its handler. */
function describeAction(action: ShortcutAction): string {
  const { handler: _handler, ...rest } = action;
  return JSON.stringify(rest);
}

interface KeyboardShortcutsContextType {
  /** Current platform */
  platform: Platform;
  /** Shortcut configuration */
  config: ShortcutConfig;
  /** Update configuration */
  setConfig: (config: Partial<ShortcutConfig>) => void;
  /** Reset to defaults */
  resetConfig: () => void;
  /** Register a shortcut action */
  registerShortcut: (action: ShortcutAction) => void;
  /** Unregister a shortcut action */
  unregisterShortcut: (actionId: string) => void;
  /** Get all registered shortcuts */
  getShortcuts: () => ShortcutAction[];
  /** Get shortcuts by category */
  getShortcutsByCategory: (category: string) => ShortcutAction[];
  /** Customize a shortcut binding */
  customizeBinding: (actionId: string, binding: KeyBinding) => void;
  /** Enable/disable a shortcut */
  toggleShortcut: (actionId: string, enabled: boolean) => void;
  /** Show keyboard shortcuts help overlay */
  showHelp: () => void;
  /** Hide keyboard shortcuts help overlay */
  hideHelp: () => void;
  /** Is help overlay visible */
  isHelpVisible: boolean;
}

const KeyboardShortcutsContext = createContext<KeyboardShortcutsContextType | undefined>(undefined);

interface KeyboardShortcutsProviderProps {
  children: React.ReactNode;
}

export function KeyboardShortcutsProvider({ children }: KeyboardShortcutsProviderProps) {
  const [platform] = useState<Platform>(() => detectPlatform());
  const [config, setConfigState] = useLocalStorage<ShortcutConfig>(STORAGE_KEY, DEFAULT_CONFIG);
  const [isHelpVisible, setIsHelpVisible] = useState(false);
  const registryRef = useRef<ShortcutRegistry>(new ShortcutRegistry(platform));
  // Bumped whenever the registry changes. The registry lives in a ref, so
  // without this the memoized context value never changes and consumers that
  // list shortcuts (help overlay, customizer) keep rendering a stale list.
  const [registryVersion, setRegistryVersion] = useState(0);

  const setConfig = useCallback((partial: Partial<ShortcutConfig>) => {
    setConfigState(prev => ({ ...prev, ...partial }));
  }, [setConfigState]);

  const resetConfig = useCallback(() => {
    setConfigState(DEFAULT_CONFIG);
  }, [setConfigState]);

  const registerShortcut = useCallback((action: ShortcutAction) => {
    const previous = registryRef.current.get(action.id);
    registryRef.current.register(action);
    // Re-registering the same shortcut with a new handler (which happens on
    // every render of a component that builds its actions inline) changes
    // nothing a consumer can display, so it must not trigger a re-render.
    if (!previous || describeAction(previous) !== describeAction(action)) {
      setRegistryVersion(version => version + 1);
    }
  }, []);

  const unregisterShortcut = useCallback((actionId: string) => {
    if (!registryRef.current.get(actionId)) return;
    registryRef.current.unregister(actionId);
    setRegistryVersion(version => version + 1);
  }, []);

  // registryVersion is a dependency on purpose: a new function identity is
  // what tells consumers the registry contents changed.
  const getShortcuts = useCallback(() => {
    return registryRef.current.getAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [registryVersion]);

  const getShortcutsByCategory = useCallback((category: string) => {
    return registryRef.current.getByCategory(category);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [registryVersion]);

  const customizeBinding = useCallback((actionId: string, binding: KeyBinding) => {
    setConfig({
      customBindings: {
        ...config.customBindings,
        [actionId]: binding,
      },
    });
  }, [config.customBindings, setConfig]);

  const toggleShortcut = useCallback((actionId: string, enabled: boolean) => {
    const disabledShortcuts = enabled
      ? config.disabledShortcuts.filter(id => id !== actionId)
      : [...config.disabledShortcuts, actionId];
    
    setConfig({ disabledShortcuts });
  }, [config.disabledShortcuts, setConfig]);

  const showHelp = useCallback(() => {
    setIsHelpVisible(true);
  }, []);

  const hideHelp = useCallback(() => {
    setIsHelpVisible(false);
  }, []);

  // Global keyboard event handler
  useEffect(() => {
    if (!config.enabled) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Skip if focus is in an input field (unless shortcut explicitly allows it)
      if (isInputFocused()) return;

      const actions = registryRef.current.getAll();

      for (const action of actions) {
        // Skip disabled shortcuts
        if (config.disabledShortcuts.includes(action.id)) continue;
        if (action.enabled === false) continue;

        // Get effective binding (custom or default)
        const binding = config.customBindings[action.id] || action.defaultBinding;

        // Check if event matches binding
        if (matchesBinding(event, binding, platform)) {
          // Prevent default if specified
          if (action.preventDefault !== false) {
            event.preventDefault();
          }

          // Stop propagation if specified
          if (action.stopPropagation) {
            event.stopPropagation();
          }

          // Execute handler
          try {
            action.handler(event);
          } catch (error) {
            logger.error(`Error executing shortcut "${action.id}":`, error instanceof Error ? error : new Error(String(error)));
          }

          // Only trigger first matching shortcut
          break;
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [config, platform]);

  const value = useMemo<KeyboardShortcutsContextType>(() => ({
    platform,
    config,
    setConfig,
    resetConfig,
    registerShortcut,
    unregisterShortcut,
    getShortcuts,
    getShortcutsByCategory,
    customizeBinding,
    toggleShortcut,
    showHelp,
    hideHelp,
    isHelpVisible,
  }), [
    platform,
    config,
    setConfig,
    resetConfig,
    registerShortcut,
    unregisterShortcut,
    getShortcuts,
    getShortcutsByCategory,
    customizeBinding,
    toggleShortcut,
    showHelp,
    hideHelp,
    isHelpVisible,
  ]);

  return (
    <KeyboardShortcutsContext.Provider value={value}>
      {children}
    </KeyboardShortcutsContext.Provider>
  );
}

export function useKeyboardShortcuts(): KeyboardShortcutsContextType {
  const context = useContext(KeyboardShortcutsContext);
  if (!context) {
    throw new Error('useKeyboardShortcuts must be used within a KeyboardShortcutsProvider');
  }
  return context;
}
