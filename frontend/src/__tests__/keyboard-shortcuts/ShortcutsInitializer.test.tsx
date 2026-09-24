import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ShortcutsInitializer } from '@/components/keyboard-shortcuts/ShortcutsInitializer';
import { KeyboardShortcutsProvider, useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import { UserPreferencesProvider } from '@/contexts/UserPreferencesContext';
import { ThemeProvider } from '@/contexts/ThemeContext';
import { CommandPaletteProvider } from '@/contexts/CommandPaletteContext';

const mockPush = vi.fn();
let mockPathname = '/en/dashboard';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: mockPush }),
  usePathname: () => mockPathname,
}));

function TestWrapper({ children }: { children: React.ReactNode }) {
  return (
    <ThemeProvider>
      <UserPreferencesProvider>
        <KeyboardShortcutsProvider>
          <CommandPaletteProvider>
            {children}
          </CommandPaletteProvider>
        </KeyboardShortcutsProvider>
      </UserPreferencesProvider>
    </ThemeProvider>
  );
}

function ShortcutsCount() {
  const { getShortcuts } = useKeyboardShortcuts();
  return <div data-testid="shortcut-count">{getShortcuts().length}</div>;
}

describe('ShortcutsInitializer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockPathname = '/en/dashboard';
  });

  it('should render without errors', () => {
    render(
      <TestWrapper>
        <ShortcutsInitializer />
      </TestWrapper>
    );
  });

  it('should register default shortcuts on mount', () => {
    render(
      <TestWrapper>
        <ShortcutsInitializer />
        <ShortcutsCount />
      </TestWrapper>
    );

    const count = screen.getByTestId('shortcut-count');
    expect(Number(count.textContent)).toBeGreaterThan(0);
  });

  it('should register show-shortcuts-help shortcut', () => {
    function HelpChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      const shortcuts = getShortcuts();
      const helpShortcut = shortcuts.find(s => s.id === 'show-shortcuts-help');
      return <div data-testid="has-help-shortcut">{helpShortcut ? 'yes' : 'no'}</div>;
    }

    render(
      <TestWrapper>
        <ShortcutsInitializer />
        <HelpChecker />
      </TestWrapper>
    );

    expect(screen.getByTestId('has-help-shortcut').textContent).toBe('yes');
  });

  it('should register navigation shortcuts', () => {
    function NavChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      const shortcuts = getShortcuts();
      const navIds = ['go-to-dashboard', 'go-to-corridors', 'go-to-anchors', 'go-to-analytics'];
      const found = navIds.every(id => shortcuts.some(s => s.id === id));
      return <div data-testid="has-nav-shortcuts">{found ? 'yes' : 'no'}</div>;
    }

    render(
      <TestWrapper>
        <ShortcutsInitializer />
        <NavChecker />
      </TestWrapper>
    );

    expect(screen.getByTestId('has-nav-shortcuts').textContent).toBe('yes');
  });

  it('should register search shortcut', () => {
    function SearchChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      const shortcuts = getShortcuts();
      const found = shortcuts.some(s => s.id === 'open-search');
      return <div data-testid="has-search-shortcut">{found ? 'yes' : 'no'}</div>;
    }

    render(
      <TestWrapper>
        <ShortcutsInitializer />
        <SearchChecker />
      </TestWrapper>
    );

    expect(screen.getByTestId('has-search-shortcut').textContent).toBe('yes');
  });

  it('should register UI shortcuts (sidebar, theme, notifications)', () => {
    function UIChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      const shortcuts = getShortcuts();
      const uiIds = ['toggle-sidebar', 'toggle-theme', 'open-notifications'];
      const found = uiIds.every(id => shortcuts.some(s => s.id === id));
      return <div data-testid="has-ui-shortcuts">{found ? 'yes' : 'no'}</div>;
    }

    render(
      <TestWrapper>
        <ShortcutsInitializer />
        <UIChecker />
      </TestWrapper>
    );

    expect(screen.getByTestId('has-ui-shortcuts').textContent).toBe('yes');
  });

  it('should register refresh shortcut', () => {
    function RefreshChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      const shortcuts = getShortcuts();
      const found = shortcuts.some(s => s.id === 'refresh-data');
      return <div data-testid="has-refresh-shortcut">{found ? 'yes' : 'no'}</div>;
    }

    render(
      <TestWrapper>
        <ShortcutsInitializer />
        <RefreshChecker />
      </TestWrapper>
    );

    expect(screen.getByTestId('has-refresh-shortcut').textContent).toBe('yes');
  });

  it('should register accessibility shortcut (skip-to-content)', () => {
    function A11yChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      const shortcuts = getShortcuts();
      const found = shortcuts.some(s => s.id === 'skip-to-content');
      return <div data-testid="has-a11y-shortcut">{found ? 'yes' : 'no'}</div>;
    }

    render(
      <TestWrapper>
        <ShortcutsInitializer />
        <A11yChecker />
      </TestWrapper>
    );

    expect(screen.getByTestId('has-a11y-shortcut').textContent).toBe('yes');
  });

  it('should return null (render nothing visible)', () => {
    const { container } = render(
      <TestWrapper>
        <ShortcutsInitializer />
      </TestWrapper>
    );

    expect(container.innerHTML).toBe('');
  });

  it('should extract locale from pathname', () => {
    mockPathname = '/es/corridors';

    function LocaleChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      const shortcuts = getShortcuts();
      const dashboardShortcut = shortcuts.find(s => s.id === 'go-to-dashboard');
      return (
        <div>
          <button
            onClick={() => dashboardShortcut?.handler(new KeyboardEvent('keydown', { key: 'd' }))}
          >
            Go to Dashboard
          </button>
        </div>
      );
    }

    render(
      <TestWrapper>
        <ShortcutsInitializer />
        <LocaleChecker />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Go to Dashboard'));

    expect(mockPush).toHaveBeenCalledWith('/es/dashboard');
  });

  it('should default to en locale when pathname has no locale', () => {
    mockPathname = '/dashboard';

    function LocaleChecker() {
      const { getShortcuts } = useKeyboardShortcuts();
      const shortcuts = getShortcuts();
      const dashboardShortcut = shortcuts.find(s => s.id === 'go-to-dashboard');
      return (
        <div>
          <button
            onClick={() => dashboardShortcut?.handler(new KeyboardEvent('keydown', { key: 'd' }))}
          >
            Go to Dashboard
          </button>
        </div>
      );
    }

    render(
      <TestWrapper>
        <ShortcutsInitializer />
        <LocaleChecker />
      </TestWrapper>
    );

    fireEvent.click(screen.getByText('Go to Dashboard'));

    expect(mockPush).toHaveBeenCalledWith('/en/dashboard');
  });
});
