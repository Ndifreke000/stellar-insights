import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ThemeToggle } from '../ThemeToggle';
import { ThemeProvider } from '../../contexts/ThemeContext';

// framer-motion uses ResizeObserver in jsdom; stub it
global.ResizeObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}));

const renderWithTheme = (ui: React.ReactElement) =>
  render(<ThemeProvider>{ui}</ThemeProvider>);

describe('ThemeToggle', () => {
  it('renders a button', () => {
    renderWithTheme(<ThemeToggle />);
    expect(screen.getByRole('button')).toBeInTheDocument();
  });

  it('has an accessible aria-label', () => {
    renderWithTheme(<ThemeToggle />);
    expect(screen.getByRole('button')).toHaveAttribute('aria-label');
  });

  it('cycles theme preference on click', () => {
    renderWithTheme(<ThemeToggle />);
    const btn = screen.getByRole('button');
    const initialLabel = btn.getAttribute('aria-label');
    fireEvent.click(btn);
    expect(btn.getAttribute('aria-label')).not.toBe(initialLabel);
  });

  it('cycles through all three preferences', () => {
    renderWithTheme(<ThemeToggle />);
    const btn = screen.getByRole('button');
    const labels = new Set<string | null>();
    labels.add(btn.getAttribute('aria-label'));
    fireEvent.click(btn);
    labels.add(btn.getAttribute('aria-label'));
    fireEvent.click(btn);
    labels.add(btn.getAttribute('aria-label'));
    expect(labels.size).toBe(3);
  });
});
