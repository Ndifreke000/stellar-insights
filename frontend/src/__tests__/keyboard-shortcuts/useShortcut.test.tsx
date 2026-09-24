import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { useShortcut, useShortcuts } from '@/hooks/useShortcut';
import { KeyboardShortcutsProvider, useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import React from 'react';

function TestWrapper({ children }: { children: React.ReactNode }) {
  return (
    <KeyboardShortcutsProvider>
      {children}
    </KeyboardShortcutsProvider>
  );
}

describe('useShortcut', () => {
  it('should register a shortcut on mount', () => {
    const handler = vi.fn();

    function TestComponent() {
      const { getShortcuts } = useKeyboardShortcuts();
      const count = getShortcuts().length;

      useShortcut({
        id: 'test-use-shortcut',
        name: 'Test Shortcut',
        description: 'A test shortcut',
        category: 'actions',
        defaultBinding: { key: 'x', modifiers: ['ctrl'] },
        handler,
      });

      return <div data-testid="count">{count}</div>;
    }

    render(
      <TestWrapper>
        <TestComponent />
      </TestWrapper>
    );

    const count = screen.getByTestId('count');
    expect(Number(count.textContent)).toBe(1);
  });

  it('should trigger handler on matching keydown', () => {
    const handler = vi.fn();

    function TestComponent() {
      useShortcut({
        id: 'test-trigger',
        name: 'Test Trigger',
        description: 'A test trigger shortcut',
        category: 'actions',
        defaultBinding: { key: 'x', modifiers: ['ctrl'] },
        handler,
      });

      return <div>Test</div>;
    }

    render(
      <TestWrapper>
        <TestComponent />
      </TestWrapper>
    );

    fireEvent.keyDown(document, { key: 'x', ctrlKey: true });

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('should not trigger handler for non-matching key', () => {
    const handler = vi.fn();

    function TestComponent() {
      useShortcut({
        id: 'test-no-match',
        name: 'No Match',
        description: 'Should not trigger',
        category: 'actions',
        defaultBinding: { key: 'x', modifiers: ['ctrl'] },
        handler,
      });

      return <div>Test</div>;
    }

    render(
      <TestWrapper>
        <TestComponent />
      </TestWrapper>
    );

    fireEvent.keyDown(document, { key: 'y', ctrlKey: true });

    expect(handler).not.toHaveBeenCalled();
  });

  it('should unregister shortcut on unmount', () => {
    const handler = vi.fn();

    function TestComponent() {
      const { getShortcuts } = useKeyboardShortcuts();

      useShortcut({
        id: 'test-unmount',
        name: 'Unmount Test',
        description: 'Should be unregistered',
        category: 'actions',
        defaultBinding: { key: 'x', modifiers: ['ctrl'] },
        handler,
      });

      return <div data-testid="count">{getShortcuts().length}</div>;
    }

    const { unmount } = render(
      <TestWrapper>
        <TestComponent />
      </TestWrapper>
    );

    expect(Number(screen.getByTestId('count').textContent)).toBe(1);

    unmount();

    function CountChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      return <div data-testid="after-count">{getShortcuts().length}</div>;
    }

    render(
      <TestWrapper>
        <CountChecker />
      </TestWrapper>
    );

    expect(Number(screen.getByTestId('after-count').textContent)).toBe(0);
  });

  it('should support platform-specific bindings', () => {
    const handler = vi.fn();

    function TestComponent() {
      useShortcut({
        id: 'test-platform',
        name: 'Platform Test',
        description: 'Platform-specific shortcut',
        category: 'actions',
        defaultBinding: {
          key: 'x',
          modifiers: ['ctrl'],
          mac: { key: 'x', modifiers: ['meta'] },
        },
        handler,
      });

      return <div>Test</div>;
    }

    render(
      <TestWrapper>
        <TestComponent />
      </TestWrapper>
    );

    fireEvent.keyDown(document, { key: 'x', metaKey: true });

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('should respect enabled flag', () => {
    const handler = vi.fn();

    function TestComponent() {
      useShortcut({
        id: 'test-disabled',
        name: 'Disabled Test',
        description: 'Should not trigger when disabled',
        category: 'actions',
        defaultBinding: { key: 'x', modifiers: ['ctrl'] },
        handler,
        enabled: false,
      });

      return <div>Test</div>;
    }

    render(
      <TestWrapper>
        <TestComponent />
      </TestWrapper>
    );

    fireEvent.keyDown(document, { key: 'x', ctrlKey: true });

    expect(handler).not.toHaveBeenCalled();
  });
});

describe('useShortcuts', () => {
  it('should register multiple shortcuts at once', () => {
    const handler1 = vi.fn();
    const handler2 = vi.fn();

    function TestComponent() {
      const { getShortcuts } = useKeyboardShortcuts();

      useShortcuts([
        {
          id: 'multi-1',
          name: 'Multi 1',
          description: 'First multi shortcut',
          category: 'actions',
          defaultBinding: { key: '1', modifiers: ['ctrl'] },
          handler: handler1,
        },
        {
          id: 'multi-2',
          name: 'Multi 2',
          description: 'Second multi shortcut',
          category: 'actions',
          defaultBinding: { key: '2', modifiers: ['ctrl'] },
          handler: handler2,
        },
      ]);

      return <div data-testid="count">{getShortcuts().length}</div>;
    }

    render(
      <TestWrapper>
        <TestComponent />
      </TestWrapper>
    );

    expect(Number(screen.getByTestId('count').textContent)).toBe(2);
  });

  it('should trigger correct handler for each shortcut', () => {
    const handler1 = vi.fn();
    const handler2 = vi.fn();

    function TestComponent() {
      useShortcuts([
        {
          id: 'multi-trigger-1',
          name: 'Multi Trigger 1',
          description: 'First',
          category: 'actions',
          defaultBinding: { key: '1', modifiers: ['ctrl'] },
          handler: handler1,
        },
        {
          id: 'multi-trigger-2',
          name: 'Multi Trigger 2',
          description: 'Second',
          category: 'actions',
          defaultBinding: { key: '2', modifiers: ['ctrl'] },
          handler: handler2,
        },
      ]);

      return <div>Test</div>;
    }

    render(
      <TestWrapper>
        <TestComponent />
      </TestWrapper>
    );

    fireEvent.keyDown(document, { key: '1', ctrlKey: true });
    expect(handler1).toHaveBeenCalledTimes(1);
    expect(handler2).not.toHaveBeenCalled();

    fireEvent.keyDown(document, { key: '2', ctrlKey: true });
    expect(handler2).toHaveBeenCalledTimes(1);
  });

  it('should unregister all shortcuts on unmount', () => {
    function TestComponent() {
      const { getShortcuts } = useKeyboardShortcuts();

      useShortcuts([
        {
          id: 'unreg-1',
          name: 'Unreg 1',
          description: 'First',
          category: 'actions',
          defaultBinding: { key: '1', modifiers: ['ctrl'] },
          handler: vi.fn(),
        },
        {
          id: 'unreg-2',
          name: 'Unreg 2',
          description: 'Second',
          category: 'actions',
          defaultBinding: { key: '2', modifiers: ['ctrl'] },
          handler: vi.fn(),
        },
      ]);

      return <div data-testid="count">{getShortcuts().length}</div>;
    }

    const { unmount } = render(
      <TestWrapper>
        <TestComponent />
      </TestWrapper>
    );

    expect(Number(screen.getByTestId('count').textContent)).toBe(2);

    unmount();

    function CountChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      return <div data-testid="after-count">{getShortcuts().length}</div>;
    }

    render(
      <TestWrapper>
        <CountChecker />
      </TestWrapper>
    );

    expect(Number(screen.getByTestId('after-count').textContent)).toBe(0);
  });
});
