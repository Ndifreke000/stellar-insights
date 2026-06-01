/**
 * Keyboard Shortcuts Registry
 * Central registry for all keyboard shortcuts with conflict detection
 */

import type { ShortcutAction, ShortcutConflict, Platform } from '@/types/keyboard-shortcuts';
import { bindingsConflict } from './utils';

export class ShortcutRegistry {
  private actions = new Map<string, ShortcutAction>();
  private platform: Platform;

  constructor(platform: Platform) {
    this.platform = platform;
  }

  /**
   * Register a new shortcut action
   */
  register(action: ShortcutAction): void {
    if (this.actions.has(action.id)) {
      console.warn(`Shortcut action "${action.id}" is already registered. Overwriting.`);
    }
    this.actions.set(action.id, action);
  }

  /**
   * Register multiple shortcut actions
   */
  registerMany(actions: ShortcutAction[]): void {
    actions.forEach(action => this.register(action));
  }

  /**
   * Unregister a shortcut action
   */
  unregister(actionId: string): void {
    this.actions.delete(actionId);
  }

  /**
   * Get a shortcut action by ID
   */
  get(actionId: string): ShortcutAction | undefined {
    return this.actions.get(actionId);
  }

  /**
   * Get all registered actions
   */
  getAll(): ShortcutAction[] {
    return Array.from(this.actions.values());
  }

  /**
   * Get actions by category
   */
  getByCategory(category: string): ShortcutAction[] {
    return this.getAll().filter(action => action.category === category);
  }

  /**
   * Check for binding conflicts
   */
  detectConflicts(): ShortcutConflict[] {
    const conflicts: ShortcutConflict[] = [];
    const actions = this.getAll();

    for (let i = 0; i < actions.length; i++) {
      const action1 = actions[i];
      const conflictsWith: string[] = [];

      for (let j = i + 1; j < actions.length; j++) {
        const action2 = actions[j];
        
        if (bindingsConflict(action1.defaultBinding, action2.defaultBinding, this.platform)) {
          conflictsWith.push(action2.id);
        }
      }

      if (conflictsWith.length > 0) {
        conflicts.push({
          actionId: action1.id,
          binding: action1.defaultBinding,
          conflictsWith,
        });
      }
    }

    return conflicts;
  }

  /**
   * Clear all registered actions
   */
  clear(): void {
    this.actions.clear();
  }

  /**
   * Update platform (useful for testing or dynamic platform changes)
   */
  setPlatform(platform: Platform): void {
    this.platform = platform;
  }
}
