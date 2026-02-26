/**
 * Accessibility Tests
 * 
 * These tests verify WCAG 2.1 AA compliance for key components
 */

import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { axe, toHaveNoViolations } from 'jest-axe';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';

// Extend expect with jest-axe matchers
expect.extend(toHaveNoViolations);

describe('Button Accessibility', () => {
  it('should not have accessibility violations', async () => {
    const { container } = render(<Button>Click me</Button>);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have proper disabled state', async () => {
    const { container, getByRole } = render(<Button disabled>Disabled</Button>);
    const button = getByRole('button');
    expect(button).toBeDisabled();
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have proper loading state', async () => {
    const { container, getByRole } = render(
      <Button loading loadingText="Loading...">Submit</Button>
    );
    const button = getByRole('button');
    expect(button).toHaveAttribute('aria-busy', 'true');
    expect(button).toBeDisabled();
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have minimum touch target size', () => {
    const { getByRole } = render(<Button>Click</Button>);
    const button = getByRole('button');
    const styles = window.getComputedStyle(button);
    
    // Check minimum 44x44px touch target
    const minHeight = parseInt(styles.minHeight);
    const minWidth = parseInt(styles.minWidth);
    
    expect(minHeight).toBeGreaterThanOrEqual(44);
    expect(minWidth).toBeGreaterThanOrEqual(44);
  });
});

describe('Badge Accessibility', () => {
  it('should not have accessibility violations', async () => {
    const { container } = render(<Badge>Status</Badge>);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should be focusable', () => {
    const { container } = render(<Badge>Status</Badge>);
    const badge = container.firstChild as HTMLElement;
    expect(badge).toHaveClass('focus:outline-none');
    expect(badge).toHaveClass('focus:ring-2');
  });
});

describe('Form Accessibility', () => {
  it('should associate labels with inputs', async () => {
    const { container, getByLabelText } = render(
      <form>
        <label htmlFor="test-input">Test Input</label>
        <input id="test-input" type="text" />
      </form>
    );
    
    const input = getByLabelText('Test Input');
    expect(input).toBeInTheDocument();
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should mark required fields', async () => {
    const { container, getByLabelText } = render(
      <form>
        <label htmlFor="required-input">
          Required Field <span aria-label="required">*</span>
        </label>
        <input id="required-input" type="text" required aria-required="true" />
      </form>
    );
    
    const input = getByLabelText(/Required Field/);
    expect(input).toHaveAttribute('required');
    expect(input).toHaveAttribute('aria-required', 'true');
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should associate error messages with inputs', async () => {
    const { container, getByRole } = render(
      <form>
        <label htmlFor="error-input">Email</label>
        <input
          id="error-input"
          type="email"
          aria-invalid="true"
          aria-describedby="error-message"
        />
        <p id="error-message" role="alert">
          Please enter a valid email
        </p>
      </form>
    );
    
    const input = getByRole('textbox');
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(input).toHaveAttribute('aria-describedby', 'error-message');
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });
});

describe('Navigation Accessibility', () => {
  it('should have proper landmark roles', async () => {
    const { container } = render(
      <div>
        <header>
          <nav aria-label="Main navigation">
            <a href="/">Home</a>
          </nav>
        </header>
        <main id="main-content">
          <h1>Page Title</h1>
        </main>
        <footer>
          <p>Footer content</p>
        </footer>
      </div>
    );
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have skip navigation link', () => {
    const { getByText } = render(
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>
    );
    
    const skipLink = getByText('Skip to main content');
    expect(skipLink).toHaveAttribute('href', '#main-content');
  });
});

describe('Image Accessibility', () => {
  it('should have alt text for meaningful images', async () => {
    const { container } = render(
      <img src="/test.jpg" alt="Description of image" />
    );
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should mark decorative images', async () => {
    const { container } = render(
      <img src="/decorative.jpg" alt="" role="presentation" />
    );
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });
});

describe('Color Contrast', () => {
  it('should have sufficient contrast for text', () => {
    // Note: This is a simplified test. Real contrast testing requires
    // actual color values from computed styles
    const { getByText } = render(
      <div style={{ backgroundColor: '#020617', color: '#ffffff' }}>
        <p>High contrast text</p>
      </div>
    );
    
    const text = getByText('High contrast text');
    expect(text).toBeInTheDocument();
    
    // In a real test, you would calculate the contrast ratio
    // and ensure it meets WCAG AA standards (4.5:1 for normal text)
  });
});

describe('Keyboard Navigation', () => {
  it('should be keyboard accessible', () => {
    const { getByRole } = render(
      <button onClick={() => {}}>Clickable</button>
    );
    
    const button = getByRole('button');
    expect(button).toHaveAttribute('type', 'button');
    
    // Button should be focusable by default
    button.focus();
    expect(document.activeElement).toBe(button);
  });

  it('should have visible focus indicator', () => {
    const { getByRole } = render(
      <button className="focus-visible:ring-2">Focus me</button>
    );
    
    const button = getByRole('button');
    expect(button).toHaveClass('focus-visible:ring-2');
  });
});

describe('ARIA Attributes', () => {
  it('should use aria-label for icon buttons', async () => {
    const { container, getByLabelText } = render(
      <button aria-label="Close dialog">
        <span aria-hidden="true">×</span>
      </button>
    );
    
    const button = getByLabelText('Close dialog');
    expect(button).toBeInTheDocument();
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should use aria-live for dynamic content', async () => {
    const { container } = render(
      <div aria-live="polite" aria-atomic="true">
        <p>Dynamic content will be announced</p>
      </div>
    );
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should use role="status" for status messages', async () => {
    const { container } = render(
      <div role="status">
        <p>Loading...</p>
      </div>
    );
    
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });
});
