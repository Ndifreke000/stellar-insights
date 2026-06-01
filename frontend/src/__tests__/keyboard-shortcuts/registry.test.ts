import { describe, it, expect, beforeEach } from 'vitest';
import { ShortcutRegistry } from '@/lib/keyboard-shortcuts/registry';
import type { ShortcutAction } from '@/types/keyboard-shortcuts';

describe('ShortcutRegistry', () => {
  let registry: ShortcutRegistry;

  const mockAction1: ShortcutAction = {
    id: 'test-action-1',
    name: 'Test Action 1',
    description: 'First test action',
    category: 'navigation',
    defaultBinding: { key: 'k', modifiers: ['ctrl'] },
    handler: () => {},
  };

  const mockAction2: ShortcutAction = {
    id: 'test-action-2',
    name: 'Test Action 2',
    description: 'Second test action',
    category: 'search',
    defaultBinding: { key: 'f', modifiers: ['ctrl'] },
    handler: () => {},
  };

  const conflictingAction: ShortcutAction = {
    id: 'conflicting-action',
    name: 'Conflicting Action',
    description: 'Action with conflicting binding',
    category: 'actions',
    defaultBinding: { key: 'k', modifiers: ['ctrl'] }, // Same as mockAction1
    handler: () => {},
  };

  beforeEach(() => {
    registry = new ShortcutRegistry('windows');
  });

  describe('register', () => {
    it('should register a new action', () => {
      registry.register(mockAction1);
      expect(registry.get(mockAction1.id)).toEqual(mockAction1);
    });

    it('should overwrite existing action with same id', () => {
      registry.register(mockAction1);
      const updated = { ...mockAction1, name: 'Updated Name' };
      registry.register(updated);
      expect(registry.get(mockAction1.id)?.name).toBe('Updated Name');
    });
  });

  describe('registerMany', () => {
    it('should register multiple actions', () => {
      registry.registerMany([mockAction1, mockAction2]);
      expect(registry.getAll()).toHaveLength(2);
      expect(registry.get(mockAction1.id)).toBeDefined();
      expect(registry.get(mockAction2.id)).toBeDefined();
    });
  });

  describe('unregister', () => {
    it('should remove a registered action', () => {
      registry.register(mockAction1);
      registry.unregister(mockAction1.id);
      expect(registry.get(mockAction1.id)).toBeUndefined();
    });

    it('should handle unregistering non-existent action', () => {
      expect(() => registry.unregister('non-existent')).not.toThrow();
    });
  });

  describe('get', () => {
    it('should retrieve registered action by id', () => {
      registry.register(mockAction1);
      expect(registry.get(mockAction1.id)).toEqual(mockAction1);
    });

    it('should return undefined for non-existent action', () => {
      expect(registry.get('non-existent')).toBeUndefined();
    });
  });

  describe('getAll', () => {
    it('should return all registered actions', () => {
      registry.registerMany([mockAction1, mockAction2]);
      const all = registry.getAll();
      expect(all).toHaveLength(2);
      expect(all).toContainEqual(mockAction1);
      expect(all).toContainEqual(mockAction2);
    });

    it('should return empty array when no actions registered', () => {
      expect(registry.getAll()).toEqual([]);
    });
  });

  describe('getByCategory', () => {
    it('should return actions filtered by category', () => {
      registry.registerMany([mockAction1, mockAction2]);
      const navigationActions = registry.getByCategory('navigation');
      expect(navigationActions).toHaveLength(1);
      expect(navigationActions[0]).toEqual(mockAction1);
    });

    it('should return empty array for category with no actions', () => {
      registry.register(mockAction1);
      expect(registry.getByCategory('system')).toEqual([]);
    });
  });

  describe('detectConflicts', () => {
    it('should detect conflicting bindings', () => {
      registry.registerMany([mockAction1, conflictingAction]);
      const conflicts = registry.detectConflicts();
      expect(conflicts).toHaveLength(1);
      expect(conflicts[0].actionId).toBe(mockAction1.id);
      expect(conflicts[0].conflictsWith).toContain(conflictingAction.id);
    });

    it('should return empty array when no conflicts exist', () => {
      registry.registerMany([mockAction1, mockAction2]);
      const conflicts = registry.detectConflicts();
      expect(conflicts).toEqual([]);
    });

    it('should detect multiple conflicts for same action', () => {
      const anotherConflict: ShortcutAction = {
        id: 'another-conflict',
        name: 'Another Conflict',
        description: 'Another conflicting action',
        category: 'ui',
        defaultBinding: { key: 'k', modifiers: ['ctrl'] },
        handler: () => {},
      };
      registry.registerMany([mockAction1, conflictingAction, anotherConflict]);
      const conflicts = registry.detectConflicts();
      expect(conflicts[0].conflictsWith).toHaveLength(2);
    });
  });

  describe('clear', () => {
    it('should remove all registered actions', () => {
      registry.registerMany([mockAction1, mockAction2]);
      registry.clear();
      expect(registry.getAll()).toEqual([]);
    });
  });

  describe('setPlatform', () => {
    it('should update platform for conflict detection', () => {
      const macAction: ShortcutAction = {
        id: 'mac-action',
        name: 'Mac Action',
        description: 'Mac-specific action',
        category: 'navigation',
        defaultBinding: {
          key: 'k',
          modifiers: ['ctrl'],
          mac: { key: 'k', modifiers: ['meta'] },
        },
        handler: () => {},
      };

      const conflictOnMac: ShortcutAction = {
        id: 'conflict-on-mac',
        name: 'Conflict on Mac',
        description: 'Conflicts on Mac platform',
        category: 'search',
        defaultBinding: { key: 'k', modifiers: ['meta'] },
        handler: () => {},
      };

      registry.registerMany([macAction, conflictOnMac]);
      
      // No conflict on Windows
      registry.setPlatform('windows');
      expect(registry.detectConflicts()).toEqual([]);

      // Conflict on Mac
      registry.setPlatform('mac');
      const conflicts = registry.detectConflicts();
      expect(conflicts.length).toBeGreaterThan(0);
    });
  });
});
