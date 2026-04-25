import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Badge } from '../ui/badge';
import { Skeleton, SkeletonText, SkeletonCard } from '../ui/Skeleton';

// ─── Badge ────────────────────────────────────────────────────────────────────

describe('Badge', () => {
  it('renders children', () => {
    render(<Badge>Active</Badge>);
    expect(screen.getByText('Active')).toBeInTheDocument();
  });

  it.each(['default', 'secondary', 'destructive', 'outline', 'success', 'warning'] as const)(
    'renders variant "%s" without crashing',
    (variant) => {
      const { container } = render(<Badge variant={variant}>Label</Badge>);
      expect(container.firstChild).toBeInTheDocument();
    }
  );

  it('applies extra className', () => {
    const { container } = render(<Badge className="extra-class">X</Badge>);
    expect(container.firstChild).toHaveClass('extra-class');
  });

  it('forwards HTML attributes', () => {
    render(<Badge data-testid="badge-el">X</Badge>);
    expect(screen.getByTestId('badge-el')).toBeInTheDocument();
  });
});

// ─── Skeleton ─────────────────────────────────────────────────────────────────

describe('Skeleton', () => {
  it('renders with aria-hidden', () => {
    const { container } = render(<Skeleton />);
    expect(container.firstChild).toHaveAttribute('aria-hidden', 'true');
  });

  it.each(['text', 'circle', 'rect', 'card'] as const)(
    'renders variant "%s"',
    (variant) => {
      const { container } = render(<Skeleton variant={variant} />);
      expect(container.firstChild).toBeInTheDocument();
    }
  );

  it('applies custom className', () => {
    const { container } = render(<Skeleton className="w-32" />);
    expect(container.firstChild).toHaveClass('w-32');
  });
});

describe('SkeletonText', () => {
  it('renders correct number of lines', () => {
    const { container } = render(<SkeletonText lines={3} />);
    // Each line is a Skeleton div with aria-hidden
    const lines = container.querySelectorAll('[aria-hidden="true"]');
    expect(lines).toHaveLength(3);
  });

  it('defaults to 1 line', () => {
    const { container } = render(<SkeletonText />);
    expect(container.querySelectorAll('[aria-hidden="true"]')).toHaveLength(1);
  });
});

describe('SkeletonCard', () => {
  it('renders without crashing', () => {
    const { container } = render(<SkeletonCard />);
    expect(container.firstChild).toBeInTheDocument();
  });
});
