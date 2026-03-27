'use client';

import React, { createContext, useContext, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ShortcutAction, ShortcutConfig, KeyBinding, Platform } from '@/types/keyboard-shortcuts';
import { ShortcutRegistry } from '@/lib/keyboard-shortcuts/registry';
import { detectPlatform, matchesBinding, isInputFocused } from '@/lib/keyboard-shortcuts/utils';
import { useLocalStorage } from '@/hooks/useLocalStorage';

const DEFAULT_CONFIG: ShortcutConfig = {
  customBindings: {},
  disabledShortcuts: [],
  enabled: true,
};

const STORAGE_KEY = 'stellar-keyboard-shortcuts';

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
  const [, forceUpdate] = useState({});

  const setConfig = useCallback((partial: Partial<ShortcutConfig>) => {
    setConfigState(prev => ({ ...prev, ...partial }));
  }, [setConfigState]);

  const resetConfig = useCallback(() => {
    setConfigState(DEFAULT_CONFIG);
  }, [setConfigState]);

  const registerShortcut = useCallback((action: ShortcutAction) => {
    registryRef.current.register(action);
    forceUpdate({});
  }, []);

  const unregisterShortcut = useCallback((actionId: string) => {
    registryRef.current.unregister(actionId);
    forceUpdate({});
  }, []);

  const getShortcuts = useCallback(() => {
    return registryRef.current.getAll();
  }, []);

  const getShortcutsByCategory = useCallback((category: string) => {
    return registryRef.current.getByCategory(category);
  }, []);

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
            console.error(`Error executing shortcut "${action.id}":`, error);
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
