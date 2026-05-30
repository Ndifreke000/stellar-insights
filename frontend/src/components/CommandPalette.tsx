'use client';

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useRouter, usePathname } from 'next/navigation';
import {
  LayoutDashboard,
  ArrowRightLeft,
  Anchor as AnchorIcon,
  BarChart3,
  Search,
  Moon,
  Sun,
  Bell,
  RefreshCw,
  Keyboard,
  X,
} from 'lucide-react';
import { useCommandPalette } from '@/contexts/CommandPaletteContext';
import { useKeyboardShortcuts } from '@/contexts/KeyboardShortcutsContext';
import { useTheme } from '@/contexts/ThemeContext';

interface Command {
  id: string;
  label: string;
  description?: string;
  icon: React.ReactNode;
  action: () => void;
  keywords: string[];
}

export function CommandPalette() {
  const { isOpen, close } = useCommandPalette();
  const { showHelp } = useKeyboardShortcuts();
  const { theme, setThemePreference } = useTheme();
  const router = useRouter();
  const pathname = usePathname();
  const locale = pathname.split('/')[1] || 'en';

  const [query, setQuery] = useState('');
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const commands = useMemo<Command[]>(() => [
    {
      id: 'go-dashboard',
      label: 'Go to Dashboard',
      description: 'Main overview',
      icon: <LayoutDashboard className="w-4 h-4" aria-hidden="true" />,
      action: () => router.push(`/${locale}/dashboard`),
      keywords: ['dashboard', 'home', 'overview'],
    },
    {
      id: 'go-corridors',
      label: 'Go to Corridors',
      description: 'Payment corridors',
      icon: <ArrowRightLeft className="w-4 h-4" aria-hidden="true" />,
      action: () => router.push(`/${locale}/corridors`),
      keywords: ['corridors', 'payments', 'routes'],
    },
    {
      id: 'go-anchors',
      label: 'Go to Anchors',
      description: 'Anchor directory',
      icon: <AnchorIcon className="w-4 h-4" aria-hidden="true" />,
      action: () => router.push(`/${locale}/anchors`),
      keywords: ['anchors', 'directory'],
    },
    {
      id: 'go-analytics',
      label: 'Go to Analytics',
      description: 'Data analytics',
      icon: <BarChart3 className="w-4 h-4" aria-hidden="true" />,
      action: () => router.push(`/${locale}/analytics`),
      keywords: ['analytics', 'charts', 'data', 'stats'],
    },
    {
      id: 'toggle-theme',
      label: theme === 'dark' ? 'Switch to Light Theme' : 'Switch to Dark Theme',
      icon: theme === 'dark'
        ? <Sun className="w-4 h-4" aria-hidden="true" />
        : <Moon className="w-4 h-4" aria-hidden="true" />,
      action: () => setThemePreference(theme === 'dark' ? 'light' : 'dark'),
      keywords: ['theme', 'dark', 'light', 'mode'],
    },
    {
      id: 'open-notifications',
      label: 'Open Notifications',
      icon: <Bell className="w-4 h-4" aria-hidden="true" />,
      action: () => {
        const btn = document.querySelector<HTMLButtonElement>('[data-notification-button]');
        btn?.click();
      },
      keywords: ['notifications', 'alerts', 'bell'],
    },
    {
      id: 'refresh-data',
      label: 'Refresh Data',
      icon: <RefreshCw className="w-4 h-4" aria-hidden="true" />,
      action: () => window.location.reload(),
      keywords: ['refresh', 'reload', 'update'],
    },
    {
      id: 'keyboard-shortcuts',
      label: 'Keyboard Shortcuts',
      description: 'View all shortcuts',
      icon: <Keyboard className="w-4 h-4" aria-hidden="true" />,
      action: () => showHelp(),
      keywords: ['keyboard', 'shortcuts', 'hotkeys', 'help'],
    },
  ], [locale, router, theme, setThemePreference, showHelp]);

  const filtered = useMemo(() => {
    if (!query.trim()) return commands;
    const q = query.toLowerCase();
    return commands.filter(
      (c) =>
        c.label.toLowerCase().includes(q) ||
        c.description?.toLowerCase().includes(q) ||
        c.keywords.some((k) => k.includes(q)),
    );
  }, [commands, query]);

  // Reset state when opened
  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setActiveIndex(0);
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [isOpen]);

  // Keep active item in view
  useEffect(() => {
    const item = listRef.current?.children[activeIndex] as HTMLElement | undefined;
    item?.scrollIntoView({ block: 'nearest' });
  }, [activeIndex]);

  const runCommand = useCallback(
    (cmd: Command) => {
      close();
      cmd.action();
    },
    [close],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, filtered.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (filtered[activeIndex]) runCommand(filtered[activeIndex]);
      } else if (e.key === 'Escape') {
        close();
      }
    },
    [filtered, activeIndex, runCommand, close],
  );

  if (!isOpen) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh] px-4"
    >
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={close}
        aria-hidden="true"
      />

      {/* Panel */}
      <div className="relative w-full max-w-lg rounded-xl border border-border bg-background shadow-2xl overflow-hidden">
        {/* Search input */}
        <div className="flex items-center gap-3 px-4 py-3 border-b border-border">
          <Search className="w-4 h-4 text-muted-foreground shrink-0" aria-hidden="true" />
          <input
            ref={inputRef}
            type="text"
            role="combobox"
            aria-expanded="true"
            aria-controls="command-palette-list"
            aria-activedescendant={filtered[activeIndex] ? `cmd-${filtered[activeIndex].id}` : undefined}
            aria-autocomplete="list"
            placeholder="Search commands…"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setActiveIndex(0);
            }}
            onKeyDown={handleKeyDown}
            className="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
          <button
            onClick={close}
            aria-label="Close command palette"
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Results */}
        <ul
          id="command-palette-list"
          ref={listRef}
          role="listbox"
          aria-label="Commands"
          className="max-h-72 overflow-y-auto py-2"
        >
          {filtered.length === 0 ? (
            <li className="px-4 py-6 text-center text-sm text-muted-foreground" role="option" aria-selected="false">
              No commands found
            </li>
          ) : (
            filtered.map((cmd, i) => (
              <li
                key={cmd.id}
                id={`cmd-${cmd.id}`}
                role="option"
                aria-selected={i === activeIndex}
                className={`flex items-center gap-3 px-4 py-2.5 cursor-pointer text-sm transition-colors ${
                  i === activeIndex
                    ? 'bg-accent/10 text-foreground'
                    : 'text-muted-foreground hover:bg-accent/5 hover:text-foreground'
                }`}
                onMouseEnter={() => setActiveIndex(i)}
                onClick={() => runCommand(cmd)}
              >
                <span className="shrink-0 text-muted-foreground">{cmd.icon}</span>
                <span className="flex-1 font-medium">{cmd.label}</span>
                {cmd.description && (
                  <span className="text-xs text-muted-foreground">{cmd.description}</span>
                )}
              </li>
            ))
          )}
        </ul>

        {/* Footer hint */}
        <div className="flex items-center gap-4 px-4 py-2 border-t border-border text-xs text-muted-foreground">
          <span><kbd className="font-mono">↑↓</kbd> navigate</span>
          <span><kbd className="font-mono">↵</kbd> select</span>
          <span><kbd className="font-mono">Esc</kbd> close</span>
        </div>
      </div>
    </div>
  );
}
