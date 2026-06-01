/**
 * Keyboard Shortcuts Type Definitions
 * Cross-platform keyboard shortcut system with accessibility support
 */

export type ModifierKey = 'ctrl' | 'alt' | 'shift' | 'meta';
export type KeyboardKey = string;

/**
 * Platform-specific modifier key mappings
 * - macOS: Cmd (⌘) = meta, Ctrl = ctrl, Option (⌥) = alt, Shift (⇧) = shift
 * - Windows/Linux: Ctrl = ctrl, Alt = alt, Shift = shift, Win/Super = meta
 */
export interface KeyBinding {
  /** Primary key (e.g., 'k', 'Enter', 'ArrowUp') */
  key: KeyboardKey;
  /** Modifier keys required */
  modifiers?: ModifierKey[];
  /** Platform-specific override (optional) */
  mac?: { key: KeyboardKey; modifiers?: ModifierKey[] };
  windows?: { key: KeyboardKey; modifiers?: ModifierKey[] };
  linux?: { key: KeyboardKey; modifiers?: ModifierKey[] };
}

export interface ShortcutAction {
  /** Unique identifier for the action */
  id: string;
  /** Human-readable name */
  name: string;
  /** Detailed description */
  description: string;
  /** Category for grouping in help overlay */
  category: ShortcutCategory;
  /** Default key binding */
  defaultBinding: KeyBinding;
  /** Handler function */
  handler: (event: KeyboardEvent) => void | Promise<void>;
  /** Whether shortcut is enabled */
  enabled?: boolean;
  /** Whether to prevent default browser behavior */
  preventDefault?: boolean;
  /** Whether to stop event propagation */
  stopPropagation?: boolean;
}

export type ShortcutCategory =
  | 'navigation'
  | 'search'
  | 'actions'
  | 'ui'
  | 'accessibility'
  | 'system';

export interface ShortcutConfig {
  /** User-customized key bindings (maps action ID to custom binding) */
  customBindings: Record<string, KeyBinding>;
  /** Disabled shortcut IDs */
  disabledShortcuts: string[];
  /** Whether shortcuts are globally enabled */
  enabled: boolean;
}

export interface ShortcutConflict {
  actionId: string;
  binding: KeyBinding;
  conflictsWith: string[];
}

/**
 * Platform detection
 */
export type Platform = 'mac' | 'windows' | 'linux' | 'unknown';

/**
 * Keyboard event context - determines if shortcuts should be active
 */
export interface KeyboardContext {
  /** Is focus in an input field? */
  isInputFocused: boolean;
  /** Is focus in a contenteditable element? */
  isContentEditableFocused: boolean;
  /** Is a modal open? */
  isModalOpen: boolean;
  /** Custom context flags */
  customFlags?: Record<string, boolean>;
}
