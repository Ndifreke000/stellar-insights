/**
 * Keyboard Shortcuts Module
 * Exports all keyboard shortcuts functionality
 */

// Types
export type {
  ModifierKey,
  KeyboardKey,
  KeyBinding,
  ShortcutAction,
  ShortcutCategory,
  ShortcutConfig,
  ShortcutConflict,
  Platform,
  KeyboardContext,
} from '@/types/keyboard-shortcuts';

// Context and Hooks
export {
  KeyboardShortcutsProvider,
  useKeyboardShortcuts,
} from '@/contexts/KeyboardShortcutsContext';

export { useShortcut, useShortcuts } from '@/hooks/useShortcut';

// Components
export { ShortcutHelpOverlay } from '@/components/keyboard-shortcuts/ShortcutHelpOverlay';
export { ShortcutCustomizer } from '@/components/keyboard-shortcuts/ShortcutCustomizer';
export { ShortcutsInitializer } from '@/components/keyboard-shortcuts/ShortcutsInitializer';

// Utilities
export {
  detectPlatform,
  getPlatformBinding,
  normalizeKey,
  matchesBinding,
  bindingsConflict,
  formatBinding,
  isInputFocused,
  conflictsWithBrowser,
} from './utils';

// Registry
export { ShortcutRegistry } from './registry';

// Default Shortcuts
export { createDefaultShortcuts } from './default-shortcuts';
