'use client';

import React, { useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import { useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import { formatBinding } from '@/lib/keyboard-shortcuts/utils';
import type { ShortcutCategory } from '@/types/keyboard-shortcuts';

const CATEGORY_LABELS: Record<ShortcutCategory, string> = {
  navigation: 'Navigation',
  search: 'Search',
  actions: 'Actions',
  ui: 'User Interface',
  accessibility: 'Accessibility',
  system: 'System',
};

export function ShortcutHelpOverlay() {
  const { isHelpVisible, hideHelp, getShortcuts, platform, config } = useKeyboardShortcuts();
  const overlayRef = useRef<HTMLDivElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);

  // Focus management
  useEffect(() => {
    if (isHelpVisible) {
      // Save previously focused element
      const previouslyFocused = document.activeElement as HTMLElement;
      
      // Focus close button
      closeButtonRef.current?.focus();
      
      // Trap focus within overlay
      const handleTab = (e: KeyboardEvent) => {
        if (e.key !== 'Tab') return;
        
        const focusableElements = overlayRef.current?.querySelectorAll<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
        
        if (!focusableElements || focusableElements.length === 0) return;
        
        const firstElement = focusableElements[0];
        const lastElement = focusableElements[focusableElements.length - 1];
        
        if (e.shiftKey && document.activeElement === firstElement) {
          e.preventDefault();
          lastElement.focus();
        } else if (!e.shiftKey && document.activeElement === lastElement) {
          e.preventDefault();
          firstElement.focus();
        }
      };
      
      document.addEventListener('keydown', handleTab);
      
      return () => {
        document.removeEventListener('keydown', handleTab);
        // Restore focus
        previouslyFocused?.focus();
      };
    }
  }, [isHelpVisible]);

  // Close on Escape
  useEffect(() => {
    if (!isHelpVisible) return;
    
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        hideHelp();
      }
    };
    
    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [isHelpVisible, hideHelp]);

  if (!isHelpVisible) return null;

  const shortcuts = getShortcuts();
  
  // Group shortcuts by category
  const shortcutsByCategory = shortcuts.reduce((acc, shortcut) => {
    if (!acc[shortcut.category]) {
      acc[shortcut.category] = [];
    }
    acc[shortcut.category].push(shortcut);
    return acc;
  }, {} as Record<ShortcutCategory, typeof shortcuts>);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-labelledby="shortcut-help-title"
      onClick={hideHelp}
    >
      <div
        ref={overlayRef}
        className="relative w-full max-w-4xl max-h-[90vh] bg-background border border-border rounded-lg shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="sticky top-0 z-10 flex items-center justify-between px-6 py-4 bg-background border-b border-border">
          <h2 id="shortcut-help-title" className="text-2xl font-semibold text-foreground">
            Keyboard Shortcuts
          </h2>
          <button
            ref={closeButtonRef}
            onClick={hideHelp}
            className="p-2 rounded-lg hover:bg-accent/10 transition-colors focus:outline-none focus:ring-2 focus:ring-accent"
            aria-label="Close keyboard shortcuts help"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="overflow-y-auto max-h-[calc(90vh-80px)] px-6 py-4">
          {Object.entries(shortcutsByCategory).map(([category, categoryShortcuts]) => (
            <div key={category} className="mb-8 last:mb-0">
              <h3 className="text-lg font-semibold text-foreground mb-4">
                {CATEGORY_LABELS[category as ShortcutCategory]}
              </h3>
              <div className="space-y-2">
                {categoryShortcuts.map((shortcut) => {
                  const binding = config.customBindings[shortcut.id] || shortcut.defaultBinding;
                  const isDisabled = config.disabledShortcuts.includes(shortcut.id);
                  
                  return (
                    <div
                      key={shortcut.id}
                      className={`flex items-center justify-between py-3 px-4 rounded-lg bg-accent/5 ${
                        isDisabled ? 'opacity-50' : ''
                      }`}
                    >
                      <div className="flex-1">
                        <div className="font-medium text-foreground">{shortcut.name}</div>
                        <div className="text-sm text-muted-foreground mt-1">
                          {shortcut.description}
                        </div>
                      </div>
                      <div className="ml-4">
                        <kbd className="inline-flex items-center gap-1 px-3 py-1.5 text-sm font-mono font-semibold text-foreground bg-background border border-border rounded-md shadow-sm">
                          {formatBinding(binding, platform)}
                        </kbd>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}

          {shortcuts.length === 0 && (
            <div className="text-center py-12 text-muted-foreground">
              No keyboard shortcuts registered.
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="sticky bottom-0 px-6 py-4 bg-background border-t border-border text-sm text-muted-foreground">
          <p>
            Press <kbd className="px-2 py-1 text-xs font-mono bg-accent/10 border border-border rounded">Esc</kbd> to close
          </p>
        </div>
      </div>
    </div>
  );
}
