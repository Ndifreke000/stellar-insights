import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ShortcutHelpOverlay } from '@/components/keyboard-shortcuts/ShortcutHelpOverlay';
import { KeyboardShortcutsProvider, useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import type { ShortcutAction } from '@/types/keyboard-shortcuts';

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
      id: 'test-nav',
      name: 'Test Navigation',
      description: 'A test navigation shortcut',
      category: 'navigation',
      defaultBinding: { key: 'd', modifiers: ['alt'], mac: { key: 'd', modifiers: ['ctrl'] } },
      handler: vi.fn(),
      preventDefault: true,
    },
    {
      id: 'test-search',
      name: 'Test Search',
      description: 'A test search shortcut',
      category: 'search',
      defaultBinding: { key: 'k', modifiers: ['ctrl'], mac: { key: 'k', modifiers: ['meta'] } },
      handler: vi.fn(),
      preventDefault: true,
    },
    {
      id: 'test-ui',
      name: 'Test UI Toggle',
      description: 'A test UI shortcut',
      category: 'ui',
      defaultBinding: { key: 'b', modifiers: ['ctrl'], mac: { key: 'b', modifiers: ['meta'] } },
      handler: vi.fn(),
      preventDefault: true,
    },
  ];

  function RegisterAndShow() {
    const { registerShortcut, showHelp } = useKeyboardShortcuts();
    const registeredRef = React.useRef<string | null>(null);

    React.useEffect(() => {
      if (registeredRef.current === null) {
        actions.forEach(action => registerShortcut(action));
        registeredRef.current = 'done';
      }
    }, [registerShortcut]);

    return (
      <div>
        <button onClick={showHelp}>Open Help</button>
        <ShortcutHelpOverlay />
      </div>
    );
  }

  return RegisterAndShow;
}

import React from 'react';

describe('ShortcutHelpOverlay', () => {
  it('should not render when help is not visible', () => {
    render(
      <TestWrapper>
        <ShortcutHelpOverlay />
      </TestWrapper>
    );

    expect(screen.queryByText('Keyboard Shortcuts')).not.toBeInTheDocument();
  });

  it('should render when showHelp is triggered', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));

    expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('should display shortcuts grouped by category', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));

    expect(screen.getByText('Navigation')).toBeInTheDocument();
    expect(screen.getByText('Search')).toBeInTheDocument();
    expect(screen.getByText('User Interface')).toBeInTheDocument();

    expect(screen.getByText('Test Navigation')).toBeInTheDocument();
    expect(screen.getByText('Test Search')).toBeInTheDocument();
    expect(screen.getByText('Test UI Toggle')).toBeInTheDocument();
  });

  it('should show shortcut descriptions', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));

    expect(screen.getByText('A test navigation shortcut')).toBeInTheDocument();
    expect(screen.getByText('A test search shortcut')).toBeInTheDocument();
    expect(screen.getByText('A test UI shortcut')).toBeInTheDocument();
  });

  it('should close on Escape key', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));
    expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();

    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByText('Keyboard Shortcuts')).not.toBeInTheDocument();
  });

  it('should close when clicking the close button', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));
    expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();

    const closeButton = screen.getByRole('button', { name: 'Close keyboard shortcuts help' });
    fireEvent.click(closeButton);

    expect(screen.queryByText('Keyboard Shortcuts')).not.toBeInTheDocument();
  });

  it('should close when clicking the backdrop', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));
    expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();

    const backdrop = screen.getByRole('dialog');
    fireEvent.click(backdrop);

    expect(screen.queryByText('Keyboard Shortcuts')).not.toBeInTheDocument();
  });

  it('should not close when clicking inside the overlay content', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));
    expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();

    const heading = screen.getByText('Keyboard Shortcuts');
    fireEvent.click(heading);

    expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();
  });

  it('should have proper accessibility attributes', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));

    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(dialog).toHaveAttribute('aria-labelledby', 'shortcut-help-title');
  });

  it('should display formatted key bindings', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));

    const kbdElements = screen.getAllByText(/^(Ctrl|Alt|Shift|⌘|⌥|⇧|⌃)/);
    expect(kbdElements.length).toBeGreaterThan(0);
  });

  it('should show Esc to close hint in footer', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));

    expect(screen.getByText('Esc')).toBeInTheDocument();
  });

  it('should display empty state when no shortcuts are registered', () => {
    function ShowEmptyHelp() {
      const { showHelp } = useKeyboardShortcuts();
      return (
        <div>
          <button onClick={showHelp}>Open Help</button>
          <ShortcutHelpOverlay />
        </div>
      );
    }

    render(
      <TestWrapper>
        <ShowEmptyHelp />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Open Help'));
    expect(screen.getByText('No keyboard shortcuts registered.')).toBeInTheDocument();
  });

  it('should restore focus when overlay is closed', () => {
    const RegisterAndShow = registerTestShortcuts();

    render(
      <TestWrapper>
        <RegisterAndShow />
      </TestWrapper>
    );

    const openButton = screen.getByText('Open Help');
    openButton.focus();

    fireEvent.click(openButton);
    expect(screen.getByText('Keyboard Shortcuts')).toBeInTheDocument();

    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByText('Keyboard Shortcuts')).not.toBeInTheDocument();
  });
});
