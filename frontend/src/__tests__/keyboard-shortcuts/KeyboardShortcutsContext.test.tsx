import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { KeyboardShortcutsProvider, useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import type { ShortcutAction } from '@/types/keyboard-shortcuts';

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
  };
})();

Object.defineProperty(window, 'localStorage', { value: localStorageMock });

// Test component that uses the context
function TestComponent() {
  const {
    platform,
    config,
    registerShortcut,
    getShortcuts,
    customizeBinding,
    toggleShortcut,
    showHelp,
    hideHelp,
    isHelpVisible,
  } = useKeyboardShortcuts();

  const handleRegister = () => {
    const action: ShortcutAction = {
      id: 'test-shortcut',
      name: 'Test Shortcut',
      description: 'A test shortcut',
      category: 'navigation',
      defaultBinding: { key: 'k', modifiers: ['ctrl'] },
      handler: () => {
        const output = document.getElementById('output');
        if (output) output.textContent = 'Shortcut triggered!';
      },
    };
    registerShortcut(action);
  };

  return (
    <div>
      <div data-testid="platform">{platform}</div>
      <div data-testid="shortcuts-count">{getShortcuts().length}</div>
      <div data-testid="help-visible">{isHelpVisible ? 'visible' : 'hidden'}</div>
      <div data-testid="enabled">{config.enabled ? 'enabled' : 'disabled'}</div>
      <button onClick={handleRegister}>Register Shortcut</button>
      <button onClick={showHelp}>Show Help</button>
      <button onClick={hideHelp}>Hide Help</button>
      <button onClick={() => toggleShortcut('test-shortcut', false)}>Disable Shortcut</button>
      <button onClick={() => customizeBinding('test-shortcut', { key: 'j', modifiers: ['ctrl'] })}>
        Customize Binding
      </button>
      <div id="output"></div>
    </div>
  );
}

describe('KeyboardShortcutsContext', () => {
  beforeEach(() => {
    localStorageMock.clear();
  });

  it('should provide platform information', () => {
    render(
      <KeyboardShortcutsProvider>
        <TestComponent />
      </KeyboardShortcutsProvider>
    );

    const platform = screen.getByTestId('platform');
    expect(['mac', 'windows', 'linux', 'unknown']).toContain(platform.textContent);
  });

  it('should register shortcuts', () => {
    render(
      <KeyboardShortcutsProvider>
        <TestComponent />
      </KeyboardShortcutsProvider>
    );

    const registerButton = screen.getByText('Register Shortcut');
    fireEvent.click(registerButton);

    const count = screen.getByTestId('shortcuts-count');
    expect(count.textContent).toBe('1');
  });

  it('should trigger shortcut handler on key press', async () => {
    render(
      <KeyboardShortcutsProvider>
        <TestComponent />
      </KeyboardShortcutsProvider>
    );

    // Register shortcut
    const registerButton = screen.getByText('Register Shortcut');
    fireEvent.click(registerButton);

    // Trigger shortcut
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true });

    await waitFor(() => {
      const output = document.getElementById('output');
      expect(output?.textContent).toBe('Shortcut triggered!');
    });
  });

  it('should not trigger disabled shortcuts', async () => {
    render(
      <KeyboardShortcutsProvider>
        <TestComponent />
      </KeyboardShortcutsProvider>
    );

    // Register and disable shortcut
    fireEvent.click(screen.getByText('Register Shortcut'));
    fireEvent.click(screen.getByText('Disable Shortcut'));

    // Try to trigger shortcut
    fireEvent.keyDown(document, { key: 'k', ctrlKey: true });

    await waitFor(() => {
      const output = document.getElementById('output');
      expect(output?.textContent).toBe('');
    });
  });

  it('should show and hide help overlay', () => {
    render(
      <KeyboardShortcutsProvider>
        <TestComponent />
      </KeyboardShortcutsProvider>
    );

    const helpStatus = screen.getByTestId('help-visible');
    expect(helpStatus.textContent).toBe('hidden');

    fireEvent.click(screen.getByText('Show Help'));
    expect(helpStatus.textContent).toBe('visible');

    fireEvent.click(screen.getByText('Hide Help'));
    expect(helpStatus.textContent).toBe('hidden');
  });

  it('should customize shortcut bindings', () => {
    render(
      <KeyboardShortcutsProvider>
        <TestComponent />
      </KeyboardShortcutsProvider>
    );

    // Register shortcut
    fireEvent.click(screen.getByText('Register Shortcut'));

    // Customize binding
    fireEvent.click(screen.getByText('Customize Binding'));

    // Verify custom binding is stored
    const stored = JSON.parse(localStorageMock.getItem('stellar-keyboard-shortcuts') || '{}');
    expect(stored.customBindings['test-shortcut']).toBeDefined();
    expect(stored.customBindings['test-shortcut'].key).toBe('j');
  });

  it('should persist configuration to localStorage', () => {
    render(
      <KeyboardShortcutsProvider>
        <TestComponent />
      </KeyboardShortcutsProvider>
    );

    fireEvent.click(screen.getByText('Register Shortcut'));
    fireEvent.click(screen.getByText('Disable Shortcut'));

    const stored = JSON.parse(localStorageMock.getItem('stellar-keyboard-shortcuts') || '{}');
    expect(stored.disabledShortcuts).toContain('test-shortcut');
  });

  it('should not trigger shortcuts when input is focused', async () => {
    render(
      <KeyboardShortcutsProvider>
        <TestComponent />
        <input data-testid="test-input" />
      </KeyboardShortcutsProvider>
    );

    // Register shortcut
    fireEvent.click(screen.getByText('Register Shortcut'));

    // Focus input
    const input = screen.getByTestId('test-input');
    input.focus();

    // Try to trigger shortcut
    fireEvent.keyDown(input, { key: 'k', ctrlKey: true });

    await waitFor(() => {
      const output = document.getElementById('output');
      expect(output?.textContent).toBe('');
    });
  });
});
