/**
 * Keyboard Shortcuts Utilities
 * Platform detection, key normalization, and binding comparison
 */

import type { KeyBinding, Platform, ModifierKey } from '@/types/keyboard-shortcuts';

/**
 * Detect the current platform
 */
export function detectPlatform(): Platform {
  if (typeof window === 'undefined') return 'unknown';
  
  const ua = window.navigator.userAgent.toLowerCase();
  const platform = window.navigator.platform?.toLowerCase() || '';
  
  if (platform.includes('mac') || ua.includes('mac')) return 'mac';
  if (platform.includes('win') || ua.includes('win')) return 'windows';
  if (platform.includes('linux') || ua.includes('linux')) return 'linux';
  
  return 'unknown';
}

/**
 * Get the platform-specific binding for a key binding
 */
export function getPlatformBinding(binding: KeyBinding, platform: Platform): KeyBinding {
  if (platform === 'mac' && binding.mac) {
    return { ...binding, ...binding.mac };
  }
  if (platform === 'windows' && binding.windows) {
    return { ...binding, ...binding.windows };
  }
  if (platform === 'linux' && binding.linux) {
    return { ...binding, ...binding.linux };
  }
  return binding;
}

/**
 * Normalize key name for consistent comparison
 */
export function normalizeKey(key: string): string {
  // Convert to lowercase
  let normalized = key.toLowerCase();
  
  // Handle special cases
  const keyMap: Record<string, string> = {
    'esc': 'escape',
    'return': 'enter',
    'del': 'delete',
    'ins': 'insert',
    'up': 'arrowup',
    'down': 'arrowdown',
    'left': 'arrowleft',
    'right': 'arrowright',
    ' ': 'space',
  };
  
  return keyMap[normalized] || normalized;
}

/**
 * Check if a keyboard event matches a key binding
 */
export function matchesBinding(
  event: KeyboardEvent,
  binding: KeyBinding,
  platform: Platform
): boolean {
  const platformBinding = getPlatformBinding(binding, platform);
  
  // Normalize and compare key
  const eventKey = normalizeKey(event.key);
  const bindingKey = normalizeKey(platformBinding.key);
  
  if (eventKey !== bindingKey) return false;
  
  // Check modifiers
  const modifiers = platformBinding.modifiers || [];
  const hasCtrl = modifiers.includes('ctrl');
  const hasAlt = modifiers.includes('alt');
  const hasShift = modifiers.includes('shift');
  const hasMeta = modifiers.includes('meta');
  
  return (
    event.ctrlKey === hasCtrl &&
    event.altKey === hasAlt &&
    event.shiftKey === hasShift &&
    event.metaKey === hasMeta
  );
}

/**
 * Check if two bindings conflict (same key combination)
 */
export function bindingsConflict(
  binding1: KeyBinding,
  binding2: KeyBinding,
  platform: Platform
): boolean {
  const b1 = getPlatformBinding(binding1, platform);
  const b2 = getPlatformBinding(binding2, platform);
  
  if (normalizeKey(b1.key) !== normalizeKey(b2.key)) return false;
  
  const mods1 = (b1.modifiers || []).sort();
  const mods2 = (b2.modifiers || []).sort();
  
  if (mods1.length !== mods2.length) return false;
  
  return mods1.every((mod, i) => mod === mods2[i]);
}

/**
 * Format a key binding for display
 */
export function formatBinding(binding: KeyBinding, platform: Platform): string {
  const platformBinding = getPlatformBinding(binding, platform);
  const modifiers = platformBinding.modifiers || [];
  
  // Platform-specific modifier symbols
  const modifierSymbols: Record<Platform, Record<ModifierKey, string>> = {
    mac: {
      ctrl: '⌃',
      alt: '⌥',
      shift: '⇧',
      meta: '⌘',
    },
    windows: {
      ctrl: 'Ctrl',
      alt: 'Alt',
      shift: 'Shift',
      meta: 'Win',
    },
    linux: {
      ctrl: 'Ctrl',
      alt: 'Alt',
      shift: 'Shift',
      meta: 'Super',
    },
    unknown: {
      ctrl: 'Ctrl',
      alt: 'Alt',
      shift: 'Shift',
      meta: 'Meta',
    },
  };
  
  const symbols = modifierSymbols[platform];
  
  // Order modifiers consistently: Ctrl/Cmd, Alt, Shift
  const orderedModifiers: ModifierKey[] = ['meta', 'ctrl', 'alt', 'shift'];
  const parts: string[] = [];
  
  for (const mod of orderedModifiers) {
    if (modifiers.includes(mod)) {
      parts.push(symbols[mod]);
    }
  }
  
  // Format key name
  const keyName = platformBinding.key.length === 1
    ? platformBinding.key.toUpperCase()
    : platformBinding.key.charAt(0).toUpperCase() + platformBinding.key.slice(1);
  
  parts.push(keyName);
  
  // Join with + for non-Mac, no separator for Mac
  return platform === 'mac' ? parts.join('') : parts.join('+');
}

/**
 * Check if the current focus is in an input element
 */
export function isInputFocused(): boolean {
  if (typeof document === 'undefined') return false;
  
  const activeElement = document.activeElement;
  if (!activeElement) return false;
  
  const tagName = activeElement.tagName.toLowerCase();
  const isInput = tagName === 'input' || tagName === 'textarea' || tagName === 'select';
  const isContentEditable = activeElement.getAttribute('contenteditable') === 'true';
  
  return isInput || isContentEditable;
}

/**
 * Check if a key binding conflicts with native browser shortcuts
 */
export function conflictsWithBrowser(binding: KeyBinding, platform: Platform): boolean {
  const platformBinding = getPlatformBinding(binding, platform);
  const key = normalizeKey(platformBinding.key);
  const modifiers = platformBinding.modifiers || [];
  
  // Common browser shortcuts to avoid
  const browserShortcuts: Array<{ key: string; modifiers: ModifierKey[] }> = [
    // Navigation
    { key: 'r', modifiers: ['ctrl'] }, // Reload
    { key: 'r', modifiers: ['meta'] }, // Reload (Mac)
    { key: 't', modifiers: ['ctrl'] }, // New tab
    { key: 't', modifiers: ['meta'] }, // New tab (Mac)
    { key: 'w', modifiers: ['ctrl'] }, // Close tab
    { key: 'w', modifiers: ['meta'] }, // Close tab (Mac)
    { key: 'n', modifiers: ['ctrl'] }, // New window
    { key: 'n', modifiers: ['meta'] }, // New window (Mac)
    { key: 'q', modifiers: ['ctrl'] }, // Quit
    { key: 'q', modifiers: ['meta'] }, // Quit (Mac)
    
    // Editing
    { key: 'a', modifiers: ['ctrl'] }, // Select all
    { key: 'a', modifiers: ['meta'] }, // Select all (Mac)
    { key: 'c', modifiers: ['ctrl'] }, // Copy
    { key: 'c', modifiers: ['meta'] }, // Copy (Mac)
    { key: 'v', modifiers: ['ctrl'] }, // Paste
    { key: 'v', modifiers: ['meta'] }, // Paste (Mac)
    { key: 'x', modifiers: ['ctrl'] }, // Cut
    { key: 'x', modifiers: ['meta'] }, // Cut (Mac)
    { key: 'z', modifiers: ['ctrl'] }, // Undo
    { key: 'z', modifiers: ['meta'] }, // Undo (Mac)
    
    // Search
    { key: 'f', modifiers: ['ctrl'] }, // Find
    { key: 'f', modifiers: ['meta'] }, // Find (Mac)
    { key: 'g', modifiers: ['ctrl'] }, // Find next
    { key: 'g', modifiers: ['meta'] }, // Find next (Mac)
  ];
  
  return browserShortcuts.some(shortcut => {
    if (normalizeKey(shortcut.key) !== key) return false;
    const mods1 = shortcut.modifiers.sort();
    const mods2 = modifiers.sort();
    if (mods1.length !== mods2.length) return false;
    return mods1.every((mod, i) => mod === mods2[i]);
  });
}
