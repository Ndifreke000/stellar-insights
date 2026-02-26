/**
 * Keyboard Shortcuts Example Component
 * Demonstrates various keyboard shortcut patterns and best practices
 * 
 * This component is for documentation purposes and can be used as a reference
 * when implementing keyboard shortcuts in your own components.
 */

'use client';

import React, { useState } from 'react';
import { useShortcut, useShortcuts } from '@/hooks/useShortcut';
import { useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import { Keyboard, Info } from 'lucide-react';

export function ShortcutExample() {
  const [count, setCount] = useState(0);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [selectedItem, setSelectedItem] = useState<number | null>(null);
  const { showHelp, platform } = useKeyboardShortcuts();

  // Example 1: Simple shortcut
  useShortcut({
    id: 'example-increment',
    name: 'Increment Counter',
    description: 'Increment the counter by 1',
    category: 'actions',
    defaultBinding: { key: '+', modifiers: ['ctrl'] },
    handler: () => setCount(prev => prev + 1),
  });

  // Example 2: Platform-specific shortcut
  useShortcut({
    id: 'example-decrement',
    name: 'Decrement Counter',
    description: 'Decrement the counter by 1',
    category: 'actions',
    defaultBinding: {
      key: '-',
      modifiers: ['ctrl'],
      mac: { key: '-', modifiers: ['meta'] }, // Cmd+- on Mac
    },
    handler: () => setCount(prev => prev - 1),
  });

  // Example 3: Multiple shortcuts at once
  useShortcuts([
    {
      id: 'example-reset',
      name: 'Reset Counter',
      description: 'Reset the counter to zero',
      category: 'actions',
      defaultBinding: { key: '0', modifiers: ['ctrl'] },
      handler: () => setCount(0),
    },
    {
      id: 'example-double',
      name: 'Double Counter',
      description: 'Double the current counter value',
      category: 'actions',
      defaultBinding: { key: '*', modifiers: ['ctrl'] },
      handler: () => setCount(prev => prev * 2),
    },
  ]);

  // Example 4: Conditional shortcut (only active when modal is open)
  useShortcut({
    id: 'example-close-modal',
    name: 'Close Modal',
    description: 'Close the example modal',
    category: 'ui',
    defaultBinding: { key: 'Escape' },
    handler: () => setIsModalOpen(false),
    enabled: isModalOpen,
  });

  // Example 5: Conditional shortcut (only active when item is selected)
  useShortcut({
    id: 'example-delete-item',
    name: 'Delete Selected Item',
    description: 'Delete the currently selected item',
    category: 'actions',
    defaultBinding: { key: 'Delete' },
    handler: () => {
      if (selectedItem !== null) {
        console.log(`Deleting item ${selectedItem}`);
        setSelectedItem(null);
      }
    },
    enabled: selectedItem !== null,
  });

  // Example 6: Async handler
  useShortcut({
    id: 'example-save',
    name: 'Save Data',
    description: 'Save the current state (simulated)',
    category: 'actions',
    defaultBinding: {
      key: 's',
      modifiers: ['ctrl'],
      mac: { key: 's', modifiers: ['meta'] },
    },
    handler: async () => {
      console.log('Saving...');
      await new Promise(resolve => setTimeout(resolve, 1000));
      console.log('Saved!');
    },
  });

  return (
    <div className="max-w-4xl mx-auto p-8 space-y-8">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Keyboard className="w-8 h-8 text-accent" />
        <h1 className="text-3xl font-bold">Keyboard Shortcuts Examples</h1>
      </div>

      {/* Info Banner */}
      <div className="flex items-start gap-3 p-4 bg-blue-500/10 border border-blue-500/20 rounded-lg">
        <Info className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />
        <div className="text-sm text-muted-foreground">
          <p className="font-medium text-foreground mb-1">Try these shortcuts:</p>
          <ul className="space-y-1">
            <li>Press <kbd className="px-2 py-1 text-xs font-mono bg-accent/10 border border-border rounded">Shift+?</kbd> to see all shortcuts</li>
            <li>Press <kbd className="px-2 py-1 text-xs font-mono bg-accent/10 border border-border rounded">{platform === 'mac' ? 'Cmd' : 'Ctrl'}++</kbd> to increment</li>
            <li>Press <kbd className="px-2 py-1 text-xs font-mono bg-accent/10 border border-border rounded">{platform === 'mac' ? 'Cmd' : 'Ctrl'}+-</kbd> to decrement</li>
            <li>Press <kbd className="px-2 py-1 text-xs font-mono bg-accent/10 border border-border rounded">{platform === 'mac' ? 'Cmd' : 'Ctrl'}+0</kbd> to reset</li>
          </ul>
        </div>
      </div>

      {/* Counter Example */}
      <div className="glass-card p-6 rounded-2xl">
        <h2 className="text-xl font-semibold mb-4">Counter Example</h2>
        <div className="flex items-center justify-center gap-4">
          <button
            onClick={() => setCount(prev => prev - 1)}
            className="px-4 py-2 bg-accent/10 hover:bg-accent/20 rounded-lg transition-colors"
          >
            -
          </button>
          <div className="text-4xl font-bold w-24 text-center">{count}</div>
          <button
            onClick={() => setCount(prev => prev + 1)}
            className="px-4 py-2 bg-accent/10 hover:bg-accent/20 rounded-lg transition-colors"
          >
            +
          </button>
        </div>
        <div className="mt-4 text-center text-sm text-muted-foreground">
          Use keyboard shortcuts to control the counter
        </div>
      </div>

      {/* Modal Example */}
      <div className="glass-card p-6 rounded-2xl">
        <h2 className="text-xl font-semibold mb-4">Modal Example</h2>
        <button
          onClick={() => setIsModalOpen(true)}
          className="px-4 py-2 bg-accent hover:bg-accent/90 text-white rounded-lg transition-colors"
        >
          Open Modal
        </button>
        <div className="mt-2 text-sm text-muted-foreground">
          Press <kbd className="px-2 py-1 text-xs font-mono bg-accent/10 border border-border rounded">Esc</kbd> to close when open
        </div>
      </div>

      {/* Selection Example */}
      <div className="glass-card p-6 rounded-2xl">
        <h2 className="text-xl font-semibold mb-4">Selection Example</h2>
        <div className="space-y-2">
          {[1, 2, 3, 4, 5].map(item => (
            <div
              key={item}
              onClick={() => setSelectedItem(item)}
              className={`p-3 rounded-lg cursor-pointer transition-colors ${
                selectedItem === item
                  ? 'bg-accent text-white'
                  : 'bg-accent/10 hover:bg-accent/20'
              }`}
            >
              Item {item}
            </div>
          ))}
        </div>
        <div className="mt-4 text-sm text-muted-foreground">
          Select an item and press <kbd className="px-2 py-1 text-xs font-mono bg-accent/10 border border-border rounded">Delete</kbd> to remove it
        </div>
      </div>

      {/* Help Button */}
      <div className="flex justify-center">
        <button
          onClick={showHelp}
          className="flex items-center gap-2 px-6 py-3 bg-accent hover:bg-accent/90 text-white rounded-lg transition-colors"
        >
          <Keyboard className="w-5 h-5" />
          View All Shortcuts
        </button>
      </div>

      {/* Modal */}
      {isModalOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
          onClick={() => setIsModalOpen(false)}
        >
          <div
            className="bg-background border border-border rounded-lg p-8 max-w-md"
            onClick={e => e.stopPropagation()}
          >
            <h3 className="text-2xl font-semibold mb-4">Example Modal</h3>
            <p className="text-muted-foreground mb-6">
              This modal can be closed by pressing the Escape key or clicking outside.
            </p>
            <button
              onClick={() => setIsModalOpen(false)}
              className="px-4 py-2 bg-accent hover:bg-accent/90 text-white rounded-lg transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
