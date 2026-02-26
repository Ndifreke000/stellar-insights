# Accessibility Implementation Guide

This guide provides specific code examples and implementation steps to achieve WCAG 2.1 AA compliance.

## Table of Contents
1. [Setup & Configuration](#setup--configuration)
2. [Color Contrast Fixes](#color-contrast-fixes)
3. [Keyboard Navigation](#keyboard-navigation)
4. [Screen Reader Support](#screen-reader-support)
5. [Form Accessibility](#form-accessibility)
6. [Motion & Animation](#motion--animation)
7. [Component Patterns](#component-patterns)

---

## Setup & Configuration

### 1. Install Accessibility Testing Tools

```bash
# Install ESLint accessibility plugin
npm install --save-dev eslint-plugin-jsx-a11y

# Install axe-core for runtime testing
npm install --save-dev @axe-core/react

# Install testing library for accessibility testing
npm install --save-dev @testing-library/jest-dom vitest-axe
```

### 2. Configure ESLint

Update `eslint.config.mjs`:

```javascript
import jsxA11y from 'eslint-plugin-jsx-a11y';

export default [
  {
    plugins: {
      'jsx-a11y': jsxA11y,
    },
    rules: {
      ...jsxA11y.configs.recommended.rules,
      'jsx-a11y/anchor-is-valid': 'error',
      'jsx-a11y/alt-text': 'error',
      'jsx-a11y/aria-props': 'error',
      'jsx-a11y/aria-role': 'error',
      'jsx-a11y/label-has-associated-control': 'error',
      'jsx-a11y/no-autofocus': 'warn',
    },
  },
];
```

### 3. Add Axe-Core to Development

Create `frontend/src/lib/axe-config.ts`:

```typescript
if (typeof window !== 'undefined' && process.env.NODE_ENV === 'development') {
  import('@axe-core/react').then((axe) => {
    axe.default(React, ReactDOM, 1000);
  });
}
```

---

## Color Contrast Fixes

### 1. Update CSS Variables for Better Contrast

Update `frontend/src/app/globals.css`:

```css
/* Dark Theme - Enhanced Contrast */
:root {
  --foreground: #ffffff; /* Increased from #f8fafc */
  --muted-foreground: #cbd5e1; /* Increased from #94a3b8 */
  
  /* Ensure 4.5:1 contrast for text */
  --text-primary: #ffffff;
  --text-secondary: #e2e8f0;
  --text-muted: #cbd5e1;
}

/* Light Theme - Enhanced Contrast */
html.light,
[data-theme="light"] {
  --foreground: #0f172a; /* Darkened */
  --muted-foreground: #475569; /* Darkened from #64748b */
  
  --text-primary: #0f172a;
  --text-secondary: #1e293b;
  --text-muted: #475569;
}

/* Minimum font size */
.text-xs {
  font-size: 0.75rem; /* 12px minimum */
}
```

### 2. Focus Indicator Improvements

```css
/* Enhanced focus indicators with 3:1 contrast */
*:focus-visible {
  outline: 3px solid var(--accent);
  outline-offset: 2px;
  border-radius: 4px;
}

/* High contrast focus for interactive elements */
button:focus-visible,
a:focus-visible,
input:focus-visible,
select:focus-visible,
textarea:focus-visible {
  outline: 3px solid var(--accent);
  outline-offset: 2px;
  box-shadow: 0 0 0 4px rgba(99, 102, 241, 0.2);
}

/* Skip link focus */
.skip-link:focus {
  outline: 3px solid #ffffff;
  outline-offset: 2px;
  box-shadow: 0 0 0 4px rgba(255, 255, 255, 0.3);
}
```

---

## Keyboard Navigation

### 1. Enhanced Skip Navigation

Update `frontend/src/components/SkipNavigation.tsx`:

```typescript
"use client";

import React from "react";

export function SkipNavigation() {
  return (
    <>
      <a
        href="#main-content"
        className="skip-link sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-4 focus:z-[100] focus:px-6 focus:py-3 focus:bg-accent focus:text-white focus:rounded-lg focus:outline-none focus:ring-4 focus:ring-accent/30 focus:font-semibold"
      >
        Skip to main content
      </a>
      <a
        href="#navigation"
        className="skip-link sr-only focus:not-sr-only focus:absolute focus:top-4 focus:left-48 focus:z-[100] focus:px-6 focus:py-3 focus:bg-accent focus:text-white focus:rounded-lg focus:outline-none focus:ring-4 focus:ring-accent/30 focus:font-semibold"
      >
        Skip to navigation
      </a>
    </>
  );
}
```

### 2. Keyboard-Accessible Charts

Create `frontend/src/components/charts/AccessibleChart.tsx`:

```typescript
"use client";

import React, { useState } from "react";

interface ChartDataPoint {
  label: string;
  value: number;
  description?: string;
}

interface AccessibleChartProps {
  data: ChartDataPoint[];
  title: string;
  description: string;
  children: React.ReactNode; // Visual chart component
}

export function AccessibleChart({
  data,
  title,
  description,
  children,
}: AccessibleChartProps) {
  const [showTable, setShowTable] = useState(false);

  return (
    <div className="space-y-4">
      {/* Visual Chart */}
      <div
        role="img"
        aria-label={`${title}. ${description}`}
        aria-describedby="chart-description"
      >
        {children}
      </div>

      {/* Screen Reader Description */}
      <div id="chart-description" className="sr-only">
        {description}
        {data.map((point, idx) => (
          <span key={idx}>
            {point.label}: {point.value}
            {point.description && `. ${point.description}`}
            {idx < data.length - 1 && ", "}
          </span>
        ))}
      </div>

      {/* Toggle Data Table */}
      <button
        onClick={() => setShowTable(!showTable)}
        className="text-sm text-accent hover:underline focus:outline-none focus:ring-2 focus:ring-accent"
        aria-expanded={showTable}
        aria-controls="chart-data-table"
      >
        {showTable ? "Hide" : "Show"} data table
      </button>

      {/* Accessible Data Table */}
      {showTable && (
        <table
          id="chart-data-table"
          className="w-full border-collapse border border-border"
        >
          <caption className="sr-only">{title} data</caption>
          <thead>
            <tr className="bg-muted">
              <th scope="col" className="border border-border px-4 py-2 text-left">
                Label
              </th>
              <th scope="col" className="border border-border px-4 py-2 text-right">
                Value
              </th>
            </tr>
          </thead>
          <tbody>
            {data.map((point, idx) => (
              <tr key={idx}>
                <td className="border border-border px-4 py-2">{point.label}</td>
                <td className="border border-border px-4 py-2 text-right">
                  {point.value}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
```

### 3. Modal Focus Trap

Create `frontend/src/hooks/useFocusTrap.ts`:

```typescript
import { useEffect, useRef } from 'react';

export function useFocusTrap(isActive: boolean) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isActive) return;

    const container = containerRef.current;
    if (!container) return;

    // Get all focusable elements
    const focusableElements = container.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
    );

    const firstElement = focusableElements[0];
    const lastElement = focusableElements[focusableElements.length - 1];

    // Focus first element
    firstElement?.focus();

    const handleTabKey = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;

      if (e.shiftKey) {
        // Shift + Tab
        if (document.activeElement === firstElement) {
          e.preventDefault();
          lastElement?.focus();
        }
      } else {
        // Tab
        if (document.activeElement === lastElement) {
          e.preventDefault();
          firstElement?.focus();
        }
      }
    };

    container.addEventListener('keydown', handleTabKey);

    return () => {
      container.removeEventListener('keydown', handleTabKey);
    };
  }, [isActive]);

  return containerRef;
}
```

---

## Screen Reader Support

### 1. Live Region Announcements

Create `frontend/src/components/LiveAnnouncer.tsx`:

```typescript
"use client";

import React, { createContext, useContext, useState, useCallback } from "react";

interface AnnouncerContextType {
  announce: (message: string, priority?: "polite" | "assertive") => void;
}

const AnnouncerContext = createContext<AnnouncerContextType | null>(null);

export function LiveAnnouncer({ children }: { children: React.ReactNode }) {
  const [politeMessage, setPoliteMessage] = useState("");
  const [assertiveMessage, setAssertiveMessage] = useState("");

  const announce = useCallback(
    (message: string, priority: "polite" | "assertive" = "polite") => {
      if (priority === "assertive") {
        setAssertiveMessage(message);
        setTimeout(() => setAssertiveMessage(""), 100);
      } else {
        setPoliteMessage(message);
        setTimeout(() => setPoliteMessage(""), 100);
      }
    },
    []
  );

  return (
    <AnnouncerContext.Provider value={{ announce }}>
      {children}
      <div
        role="status"
        aria-live="polite"
        aria-atomic="true"
        className="sr-only"
      >
        {politeMessage}
      </div>
      <div
        role="alert"
        aria-live="assertive"
        aria-atomic="true"
        className="sr-only"
      >
        {assertiveMessage}
      </div>
    </AnnouncerContext.Provider>
  );
}

export function useAnnouncer() {
  const context = useContext(AnnouncerContext);
  if (!context) {
    throw new Error("useAnnouncer must be used within LiveAnnouncer");
  }
  return context;
}
```

### 2. Loading State Announcements

Update loading components:

```typescript
"use client";

import React from "react";
import { Loader2 } from "lucide-react";

interface LoadingSpinnerProps {
  message?: string;
  size?: "sm" | "md" | "lg";
}

export function LoadingSpinner({ 
  message = "Loading", 
  size = "md" 
}: LoadingSpinnerProps) {
  const sizeClasses = {
    sm: "w-4 h-4",
    md: "w-6 h-6",
    lg: "w-8 h-8",
  };

  return (
    <div role="status" aria-live="polite" className="flex items-center gap-2">
      <Loader2 className={`${sizeClasses[size]} animate-spin`} aria-hidden="true" />
      <span className="sr-only">{message}</span>
    </div>
  );
}
```

---

## Form Accessibility

### 1. Accessible Form Input Component

Create `frontend/src/components/ui/FormInput.tsx`:

```typescript
"use client";

import React, { useId } from "react";

interface FormInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label: string;
  error?: string;
  helperText?: string;
  required?: boolean;
}

export function FormInput({
  label,
  error,
  helperText,
  required,
  id,
  ...props
}: FormInputProps) {
  const generatedId = useId();
  const inputId = id || generatedId;
  const errorId = `${inputId}-error`;
  const helperId = `${inputId}-helper`;

  return (
    <div className="space-y-1">
      <label
        htmlFor={inputId}
        className="block text-sm font-medium text-foreground"
      >
        {label}
        {required && (
          <span className="text-red-500 ml-1" aria-label="required">
            *
          </span>
        )}
      </label>

      {helperText && (
        <p id={helperId} className="text-sm text-muted-foreground">
          {helperText}
        </p>
      )}

      <input
        id={inputId}
        aria-invalid={error ? "true" : "false"}
        aria-describedby={
          error ? errorId : helperText ? helperId : undefined
        }
        aria-required={required}
        className={`w-full rounded-lg border px-4 py-2 focus:outline-none focus:ring-2 focus:ring-accent ${
          error
            ? "border-red-500 focus:ring-red-500"
            : "border-border focus:ring-accent"
        }`}
        {...props}
      />

      {error && (
        <p id={errorId} className="text-sm text-red-500" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
```

### 2. Accessible Select Component

```typescript
"use client";

import React, { useId } from "react";

interface FormSelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  label: string;
  options: Array<{ value: string; label: string }>;
  error?: string;
  helperText?: string;
  required?: boolean;
}

export function FormSelect({
  label,
  options,
  error,
  helperText,
  required,
  id,
  ...props
}: FormSelectProps) {
  const generatedId = useId();
  const selectId = id || generatedId;
  const errorId = `${selectId}-error`;
  const helperId = `${selectId}-helper`;

  return (
    <div className="space-y-1">
      <label
        htmlFor={selectId}
        className="block text-sm font-medium text-foreground"
      >
        {label}
        {required && (
          <span className="text-red-500 ml-1" aria-label="required">
            *
          </span>
        )}
      </label>

      {helperText && (
        <p id={helperId} className="text-sm text-muted-foreground">
          {helperText}
        </p>
      )}

      <select
        id={selectId}
        aria-invalid={error ? "true" : "false"}
        aria-describedby={
          error ? errorId : helperText ? helperId : undefined
        }
        aria-required={required}
        className={`w-full rounded-lg border px-4 py-2 focus:outline-none focus:ring-2 ${
          error
            ? "border-red-500 focus:ring-red-500"
            : "border-border focus:ring-accent"
        }`}
        {...props}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>

      {error && (
        <p id={errorId} className="text-sm text-red-500" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
```

---

## Motion & Animation

### 1. Respect Reduced Motion Preference

Update `frontend/src/app/globals.css`:

```css
/* Respect prefers-reduced-motion */
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }

  .animate-spin,
  .animate-pulse,
  .animate-pulse-slow {
    animation: none !important;
  }
}
```

### 2. Conditional Animation Hook

Create `frontend/src/hooks/useReducedMotion.ts`:

```typescript
import { useEffect, useState } from 'react';

export function useReducedMotion(): boolean {
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    setPrefersReducedMotion(mediaQuery.matches);

    const handleChange = (event: MediaQueryListEvent) => {
      setPrefersReducedMotion(event.matches);
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);

  return prefersReducedMotion;
}
```

### 3. Conditional Framer Motion

```typescript
import { useReducedMotion } from '@/hooks/useReducedMotion';
import { motion } from 'framer-motion';

export function AnimatedComponent() {
  const prefersReducedMotion = useReducedMotion();

  const variants = prefersReducedMotion
    ? {}
    : {
        initial: { opacity: 0, y: 20 },
        animate: { opacity: 1, y: 0 },
        exit: { opacity: 0, y: -20 },
      };

  return (
    <motion.div {...variants} transition={{ duration: 0.3 }}>
      Content
    </motion.div>
  );
}
```

---

## Component Patterns

### 1. Accessible Button Component

Update `frontend/src/components/ui/button.tsx`:

```typescript
import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { Loader2 } from "lucide-react";

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 disabled:cursor-not-allowed",
  {
    variants: {
      variant: {
        default: "bg-accent text-accent-foreground hover:bg-accent/90",
        destructive: "bg-red-600 text-white hover:bg-red-700",
        outline: "border-2 border-border bg-background hover:bg-accent/10",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ghost: "hover:bg-accent/10 hover:text-accent-foreground",
        link: "text-accent underline-offset-4 hover:underline",
      },
      size: {
        default: "h-10 px-4 py-2 min-w-[44px]", // Ensure minimum touch target
        sm: "h-9 rounded-md px-3 min-w-[44px]",
        lg: "h-11 rounded-md px-8 min-w-[44px]",
        icon: "h-10 w-10 min-h-[44px] min-w-[44px]", // 44x44px minimum
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
  loading?: boolean;
  loadingText?: string;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ 
    className, 
    variant, 
    size, 
    asChild = false, 
    loading = false,
    loadingText,
    children,
    disabled,
    ...props 
  }, ref) => {
    const Comp = asChild ? Slot : "button";
    
    return (
      <Comp
        className={buttonVariants({ variant, size, className })}
        ref={ref}
        disabled={disabled || loading}
        aria-busy={loading}
        {...props}
      >
        {loading && (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" aria-hidden="true" />
            <span className="sr-only">Loading</span>
          </>
        )}
        {loading && loadingText ? loadingText : children}
      </Comp>
    );
  }
);
Button.displayName = "Button";

export { Button, buttonVariants };
```

### 2. Accessible Modal/Dialog

Create `frontend/src/components/ui/Dialog.tsx`:

```typescript
"use client";

import React, { useEffect } from "react";
import { X } from "lucide-react";
import { useFocusTrap } from "@/hooks/useFocusTrap";

interface DialogProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  children: React.ReactNode;
}

export function Dialog({
  isOpen,
  onClose,
  title,
  description,
  children,
}: DialogProps) {
  const containerRef = useFocusTrap(isOpen);

  useEffect(() => {
    if (!isOpen) return;

    // Prevent body scroll
    document.body.style.overflow = "hidden";

    // Handle Escape key
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };

    document.addEventListener("keydown", handleEscape);

    return () => {
      document.body.style.overflow = "";
      document.removeEventListener("keydown", handleEscape);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      role="dialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
      aria-describedby={description ? "dialog-description" : undefined}
    >
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Dialog Content */}
      <div
        ref={containerRef}
        className="relative z-10 w-full max-w-lg mx-4 bg-background border border-border rounded-2xl shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-border">
          <h2 id="dialog-title" className="text-xl font-semibold">
            {title}
          </h2>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-muted transition-colors focus:outline-none focus:ring-2 focus:ring-accent"
            aria-label="Close dialog"
          >
            <X className="w-5 h-5" aria-hidden="true" />
          </button>
        </div>

        {/* Description */}
        {description && (
          <p id="dialog-description" className="px-6 pt-4 text-muted-foreground">
            {description}
          </p>
        )}

        {/* Content */}
        <div className="p-6">{children}</div>
      </div>
    </div>
  );
}
```

---

## Testing Checklist

### Automated Tests

```typescript
// Example accessibility test
import { render } from '@testing-library/react';
import { axe, toHaveNoViolations } from 'jest-axe';
import { Button } from '@/components/ui/button';

expect.extend(toHaveNoViolations);

describe('Button Accessibility', () => {
  it('should not have accessibility violations', async () => {
    const { container } = render(<Button>Click me</Button>);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('should have proper ARIA attributes when loading', () => {
    const { getByRole } = render(<Button loading>Submit</Button>);
    const button = getByRole('button');
    expect(button).toHaveAttribute('aria-busy', 'true');
    expect(button).toBeDisabled();
  });
});
```

---

## Next Steps

1. Install and configure testing tools
2. Run automated accessibility audits
3. Fix critical issues (P0)
4. Implement high-priority improvements (P1)
5. Conduct manual testing with screen readers
6. User testing with assistive technology users
7. Document accessibility features for users

---

**Last updated:** February 23, 2026
