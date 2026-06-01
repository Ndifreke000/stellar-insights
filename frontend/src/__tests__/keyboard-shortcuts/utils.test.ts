import { describe, it, expect } from 'vitest';
import {
  detectPlatform,
  getPlatformBinding,
  normalizeKey,
  matchesBinding,
  bindingsConflict,
  formatBinding,
  isInputFocused,
  conflictsWithBrowser,
} from '@/lib/keyboard-shortcuts/utils';
import type { KeyBinding } from '@/types/keyboard-shortcuts';

describe('Keyboard Shortcuts Utils', () => {
  describe('detectPlatform', () => {
    it('should detect platform from user agent', () => {
      const platform = detectPlatform();
      expect(['mac', 'windows', 'linux', 'unknown']).toContain(platform);
    });
  });

  describe('getPlatformBinding', () => {
    it('should return default binding when no platform override exists', () => {
      const binding: KeyBinding = { key: 'k', modifiers: ['ctrl'] };
      const result = getPlatformBinding(binding, 'windows');
      expect(result).toEqual(binding);
    });

    it('should return mac-specific binding on mac platform', () => {
      const binding: KeyBinding = {
        key: 'k',
        modifiers: ['ctrl'],
        mac: { key: 'k', modifiers: ['meta'] },
      };
      const result = getPlatformBinding(binding, 'mac');
      expect(result.modifiers).toContain('meta');
    });

    it('should return windows-specific binding on windows platform', () => {
      const binding: KeyBinding = {
        key: 'k',
        modifiers: ['ctrl'],
        windows: { key: 'k', modifiers: ['ctrl', 'shift'] },
      };
      const result = getPlatformBinding(binding, 'windows');
      expect(result.modifiers).toContain('shift');
    });
  });

  describe('normalizeKey', () => {
    it('should normalize common key aliases', () => {
      expect(normalizeKey('Esc')).toBe('escape');
      expect(normalizeKey('Return')).toBe('enter');
      expect(normalizeKey('Del')).toBe('delete');
      expect(normalizeKey(' ')).toBe('space');
    });

    it('should convert to lowercase', () => {
      expect(normalizeKey('K')).toBe('k');
      expect(normalizeKey('Enter')).toBe('enter');
    });

    it('should handle arrow keys', () => {
      expect(normalizeKey('Up')).toBe('arrowup');
      expect(normalizeKey('Down')).toBe('arrowdown');
      expect(normalizeKey('Left')).toBe('arrowleft');
      expect(normalizeKey('Right')).toBe('arrowright');
    });
  });

  describe('matchesBinding', () => {
    it('should match simple key binding', () => {
      const binding: KeyBinding = { key: 'k' };
      const event = new KeyboardEvent('keydown', { key: 'k' });
      expect(matchesBinding(event, binding, 'windows')).toBe(true);
    });

    it('should match binding with modifiers', () => {
      const binding: KeyBinding = { key: 'k', modifiers: ['ctrl'] };
      const event = new KeyboardEvent('keydown', { key: 'k', ctrlKey: true });
      expect(matchesBinding(event, binding, 'windows')).toBe(true);
    });

    it('should not match when modifiers differ', () => {
      const binding: KeyBinding = { key: 'k', modifiers: ['ctrl'] };
      const event = new KeyboardEvent('keydown', { key: 'k', shiftKey: true });
      expect(matchesBinding(event, binding, 'windows')).toBe(false);
    });

    it('should match multiple modifiers', () => {
      const binding: KeyBinding = { key: 'k', modifiers: ['ctrl', 'shift'] };
      const event = new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, shiftKey: true });
      expect(matchesBinding(event, binding, 'windows')).toBe(true);
    });

    it('should use platform-specific binding', () => {
      const binding: KeyBinding = {
        key: 'k',
        modifiers: ['ctrl'],
        mac: { key: 'k', modifiers: ['meta'] },
      };
      const event = new KeyboardEvent('keydown', { key: 'k', metaKey: true });
      expect(matchesBinding(event, binding, 'mac')).toBe(true);
    });
  });

  describe('bindingsConflict', () => {
    it('should detect identical bindings as conflicts', () => {
      const binding1: KeyBinding = { key: 'k', modifiers: ['ctrl'] };
      const binding2: KeyBinding = { key: 'k', modifiers: ['ctrl'] };
      expect(bindingsConflict(binding1, binding2, 'windows')).toBe(true);
    });

    it('should not detect conflict for different keys', () => {
      const binding1: KeyBinding = { key: 'k', modifiers: ['ctrl'] };
      const binding2: KeyBinding = { key: 'j', modifiers: ['ctrl'] };
      expect(bindingsConflict(binding1, binding2, 'windows')).toBe(false);
    });

    it('should not detect conflict for different modifiers', () => {
      const binding1: KeyBinding = { key: 'k', modifiers: ['ctrl'] };
      const binding2: KeyBinding = { key: 'k', modifiers: ['shift'] };
      expect(bindingsConflict(binding1, binding2, 'windows')).toBe(false);
    });

    it('should handle platform-specific bindings', () => {
      const binding1: KeyBinding = {
        key: 'k',
        modifiers: ['ctrl'],
        mac: { key: 'k', modifiers: ['meta'] },
      };
      const binding2: KeyBinding = { key: 'k', modifiers: ['meta'] };
      expect(bindingsConflict(binding1, binding2, 'mac')).toBe(true);
    });
  });

  describe('formatBinding', () => {
    it('should format simple key', () => {
      const binding: KeyBinding = { key: 'k' };
      const result = formatBinding(binding, 'windows');
      expect(result).toBe('K');
    });

    it('should format binding with modifiers on Windows', () => {
      const binding: KeyBinding = { key: 'k', modifiers: ['ctrl'] };
      const result = formatBinding(binding, 'windows');
      expect(result).toBe('Ctrl+K');
    });

    it('should format binding with modifiers on Mac', () => {
      const binding: KeyBinding = { key: 'k', modifiers: ['meta'] };
      const result = formatBinding(binding, 'mac');
      expect(result).toBe('⌘K');
    });

    it('should format multiple modifiers in correct order', () => {
      const binding: KeyBinding = { key: 'k', modifiers: ['shift', 'ctrl', 'alt'] };
      const result = formatBinding(binding, 'windows');
      expect(result).toBe('Ctrl+Alt+Shift+K');
    });

    it('should format special keys', () => {
      const binding: KeyBinding = { key: 'Enter', modifiers: ['ctrl'] };
      const result = formatBinding(binding, 'windows');
      expect(result).toBe('Ctrl+Enter');
    });
  });

  describe('conflictsWithBrowser', () => {
    it('should detect Ctrl+R as browser shortcut', () => {
      const binding: KeyBinding = { key: 'r', modifiers: ['ctrl'] };
      expect(conflictsWithBrowser(binding, 'windows')).toBe(true);
    });

    it('should detect Cmd+T as browser shortcut on Mac', () => {
      const binding: KeyBinding = { key: 't', modifiers: ['meta'] };
      expect(conflictsWithBrowser(binding, 'mac')).toBe(true);
    });

    it('should not detect custom shortcuts as browser conflicts', () => {
      const binding: KeyBinding = { key: 'k', modifiers: ['ctrl', 'shift'] };
      expect(conflictsWithBrowser(binding, 'windows')).toBe(false);
    });

    it('should detect Ctrl+W as browser shortcut', () => {
      const binding: KeyBinding = { key: 'w', modifiers: ['ctrl'] };
      expect(conflictsWithBrowser(binding, 'windows')).toBe(true);
    });
  });
});
