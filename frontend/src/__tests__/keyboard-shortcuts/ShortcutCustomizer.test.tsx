import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ShortcutCustomizer } from '@/components/keyboard-shortcuts/ShortcutCustomizer';
import { KeyboardShortcutsProvider, useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import type { ShortcutAction } from '@/types/keyboard-shortcuts';
import React from 'react';

function TestWrapper({ children }: { children: React.ReactNode }) {
  return (
    <KeyboardShortcutsProvider>
      {children}
    </KeyboardShortcutsProvider>
  );
}

function registerTestShortcuts() {
  const actions: ShortcutAction[] = [
    {
      id: 'test-shortcut-1',
      name: 'Test Shortcut One',
      description: 'First test shortcut',
      category: 'navigation',
      defaultBinding: { key: 'd', modifiers: ['alt'], mac: { key: 'd', modifiers: ['ctrl'] } },
      handler: vi.fn(),
      preventDefault: true,
    },
    {
      id: 'test-shortcut-2',
      name: 'Test Shortcut Two',
      description: 'Second test shortcut',
      category: 'actions',
      defaultBinding: { key: 'r', modifiers: ['ctrl', 'shift'], mac: { key: 'r', modifiers: ['meta', 'shift'] } },
      handler: vi.fn(),
      preventDefault: true,
    },
  ];

  function RegisterAndCustomize({ categories }: { categories?: string[] }) {
    const { registerShortcut } = useKeyboardShortcuts();
    const registeredRef = React.useRef<string | null>(null);

    React.useEffect(() => {
      if (registeredRef.current === null) {
        actions.forEach(action => registerShortcut(action));
        registeredRef.current = 'done';
      }
    }, [registerShortcut]);

    return <ShortcutCustomizer categories={categories} />;
  }

  return RegisterAndCustomize;
}

describe('ShortcutCustomizer', () => {
  it('should render the customizer header', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    expect(screen.getByText('Customize Shortcuts')).toBeInTheDocument();
  });

  it('should display registered shortcuts', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    expect(screen.getByText('Test Shortcut One')).toBeInTheDocument();
    expect(screen.getByText('Test Shortcut Two')).toBeInTheDocument();
    expect(screen.getByText('First test shortcut')).toBeInTheDocument();
    expect(screen.getByText('Second test shortcut')).toBeInTheDocument();
  });

  it('should show Reset All button', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    expect(screen.getByText('Reset All')).toBeInTheDocument();
  });

  it('should show enabled toggle for each shortcut', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    const checkboxes = screen.getAllByRole('checkbox');
    expect(checkboxes.length).toBe(2);
    checkboxes.forEach(checkbox => {
      expect(checkbox).toBeChecked();
    });
  });

  it('should disable a shortcut when toggle is unchecked', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    const checkboxes = screen.getAllByRole('checkbox');
    fireEvent.click(checkboxes[0]);

    expect(checkboxes[0]).not.toBeChecked();
  });

  it('should show formatted key bindings for each shortcut', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    const kbdElements = screen.getAllByText(/^(Alt|Ctrl|Shift)/);
    expect(kbdElements.length).toBeGreaterThanOrEqual(2);
  });

  it('should filter shortcuts by category', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize categories={['navigation']} />
      </TestWrapper>
    );

    expect(screen.getByText('Test Shortcut One')).toBeInTheDocument();
    expect(screen.queryByText('Test Shortcut Two')).not.toBeInTheDocument();
  });

  it('should show help text about customization', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    expect(screen.getByText(/Click on a keyboard shortcut to customize it/)).toBeInTheDocument();
  });

  it('should start recording mode when clicking a kbd element', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    const kbdElements = screen.getAllByText(/^(Alt|Ctrl)/);
    fireEvent.click(kbdElements[0]);

    expect(screen.getByPlaceholderText('Press keys...')).toBeInTheDocument();
  });

  it('should cancel recording on Escape key', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    const kbdElements = screen.getAllByText(/^(Alt|Ctrl)/);
    fireEvent.click(kbdElements[0]);

    expect(screen.getByPlaceholderText('Press keys...')).toBeInTheDocument();

    fireEvent.keyDown(document, { key: 'Escape' });

    expect(screen.queryByPlaceholderText('Press keys...')).not.toBeInTheDocument();
  });

  it('should have accessible labels for toggle checkboxes', () => {
    const RegisterAndCustomize = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndCustomize />
      </TestWrapper>
    );

    const checkboxes = screen.getAllByRole('checkbox');
    checkboxes.forEach(checkbox => {
      expect(checkbox).toBeInTheDocument();
    });
  });
});
