'use client';

import React, { useState, useRef, useEffect } from 'react';
import { Keyboard, RotateCcw, AlertCircle, Check } from 'lucide-react';
import { useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import { formatBinding, conflictsWithBrowser, bindingsConflict } from '@/lib/keyboard-shortcuts/utils';
import type { KeyBinding, ModifierKey, ShortcutAction } from '@/types/keyboard-shortcuts';

interface ShortcutCustomizerProps {
  /** Optional: Only show shortcuts from specific categories */
  categories?: string[];
}

export function ShortcutCustomizer({ categories }: ShortcutCustomizerProps) {
  const {
    getShortcuts,
    getShortcutsByCategory,
    platform,
    config,
    customizeBinding,
    toggleShortcut,
    resetConfig,
  } = useKeyboardShortcuts();

  const [editingId, setEditingId] = useState<string | null>(null);
  const [recordedBinding, setRecordedBinding] = useState<KeyBinding | null>(null);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const shortcuts = categories
    ? categories.flatMap(cat => getShortcutsByCategory(cat))
    : getShortcuts();

  // Start recording a new binding
  const startRecording = (shortcutId: string) => {
    setEditingId(shortcutId);
    setRecordedBinding(null);
    setError(null);
    setTimeout(() => inputRef.current?.focus(), 0);
  };

  // Cancel recording
  const cancelRecording = () => {
    setEditingId(null);
    setRecordedBinding(null);
    setError(null);
  };

  // Save the recorded binding
  const saveBinding = (shortcutId: string) => {
    if (!recordedBinding) return;

    // Check for conflicts with other shortcuts
    const otherShortcuts = shortcuts.filter(s => s.id !== shortcutId);
    const conflicts = otherShortcuts.filter(s => {
      const binding = config.customBindings[s.id] || s.defaultBinding;
      return bindingsConflict(recordedBinding, binding, platform);
    });

    if (conflicts.length > 0) {
      setError(`Conflicts with: ${conflicts.map(s => s.name).join(', ')}`);
      return;
    }

    // Check for browser conflicts
    if (conflictsWithBrowser(recordedBinding, platform)) {
      setError('This shortcut conflicts with a browser shortcut');
      return;
    }

    customizeBinding(shortcutId, recordedBinding);
    cancelRecording();
  };

  // Reset a single shortcut to default
  const resetShortcut = (shortcutId: string) => {
    const newBindings = { ...config.customBindings };
    delete newBindings[shortcutId];
    customizeBinding(shortcutId, shortcuts.find(s => s.id === shortcutId)!.defaultBinding);
  };

  // Record keyboard input
  useEffect(() => {
    if (editingId === null) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Ignore modifier-only presses
      if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) return;

      // Escape cancels
      if (e.key === 'Escape') {
        cancelRecording();
        return;
      }

      const modifiers: ModifierKey[] = [];
      if (e.ctrlKey) modifiers.push('ctrl');
      if (e.altKey) modifiers.push('alt');
      if (e.shiftKey) modifiers.push('shift');
      if (e.metaKey) modifiers.push('meta');

      const binding: KeyBinding = {
        key: e.key,
        modifiers: modifiers.length > 0 ? modifiers : undefined,
      };

      setRecordedBinding(binding);
      setError(null);
    };

    document.addEventListener('keydown', handleKeyDown, true);
    return () => document.removeEventListener('keydown', handleKeyDown, true);
  }, [editingId]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Keyboard className="w-6 h-6 text-accent" />
          <h3 className="text-xl font-semibold text-foreground">Customize Shortcuts</h3>
        </div>
        <button
          onClick={() => resetConfig()}
          className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-foreground bg-accent/10 hover:bg-accent/20 rounded-lg transition-colors"
        >
          <RotateCcw className="w-4 h-4" />
          Reset All
        </button>
      </div>

      {/* Shortcuts List */}
      <div className="space-y-2">
        {shortcuts.map((shortcut) => {
          const binding = config.customBindings[shortcut.id] || shortcut.defaultBinding;
          const isCustomized = !!config.customBindings[shortcut.id];
          const isDisabled = config.disabledShortcuts.includes(shortcut.id);
          const isEditing = editingId === shortcut.id;

          return (
            <div
              key={shortcut.id}
              className={`flex items-center justify-between p-4 rounded-lg border ${
                isDisabled
                  ? 'bg-accent/5 border-border/50 opacity-50'
                  : 'bg-background border-border'
              }`}
            >
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-foreground">{shortcut.name}</span>
                  {isCustomized && (
                    <span className="px-2 py-0.5 text-xs font-medium text-accent bg-accent/10 rounded">
                      Custom
                    </span>
                  )}
                </div>
                <p className="text-sm text-muted-foreground mt-1">{shortcut.description}</p>
              </div>

              <div className="flex items-center gap-3 ml-4">
                {/* Enable/Disable Toggle */}
                <label className="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={!isDisabled}
                    onChange={(e) => toggleShortcut(shortcut.id, e.target.checked)}
                    className="w-4 h-4 rounded border-border text-accent focus:ring-2 focus:ring-accent"
                  />
                  <span className="text-sm text-muted-foreground">Enabled</span>
                </label>

                {/* Binding Display/Editor */}
                {isEditing ? (
                  <div className="flex items-center gap-2">
                    <input
                      ref={inputRef}
                      type="text"
                      value={recordedBinding ? formatBinding(recordedBinding, platform) : 'Press keys...'}
                      readOnly
                      className="w-40 px-3 py-2 text-sm font-mono text-center bg-accent/10 border border-accent rounded-lg focus:outline-none focus:ring-2 focus:ring-accent"
                      placeholder="Press keys..."
                    />
                    <button
                      onClick={() => saveBinding(shortcut.id)}
                      disabled={!recordedBinding}
                      className="p-2 text-green-500 hover:bg-green-500/10 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                      aria-label="Save binding"
                    >
                      <Check className="w-4 h-4" />
                    </button>
                    <button
                      onClick={cancelRecording}
                      className="p-2 text-red-500 hover:bg-red-500/10 rounded-lg transition-colors"
                      aria-label="Cancel"
                    >
                      ×
                    </button>
                  </div>
                ) : (
                  <>
                    <kbd
                      onClick={() => !isDisabled && startRecording(shortcut.id)}
                      className={`px-3 py-2 text-sm font-mono font-semibold bg-background border border-border rounded-md shadow-sm ${
                        !isDisabled ? 'cursor-pointer hover:border-accent transition-colors' : ''
                      }`}
                    >
                      {formatBinding(binding, platform)}
                    </kbd>
                    {isCustomized && (
                      <button
                        onClick={() => resetShortcut(shortcut.id)}
                        className="p-2 text-muted-foreground hover:text-foreground hover:bg-accent/10 rounded-lg transition-colors"
                        aria-label="Reset to default"
                      >
                        <RotateCcw className="w-4 h-4" />
                      </button>
                    )}
                  </>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Error Message */}
      {error && (
        <div className="flex items-center gap-2 p-4 bg-red-500/10 border border-red-500/20 rounded-lg">
          <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0" />
          <p className="text-sm text-red-500">{error}</p>
        </div>
      )}

      {/* Help Text */}
      <div className="p-4 bg-accent/5 border border-border rounded-lg">
        <p className="text-sm text-muted-foreground">
          Click on a keyboard shortcut to customize it. Press the desired key combination, then click the checkmark to save.
          Press Escape to cancel.
        </p>
      </div>
    </div>
  );
}
