# ErrorBoundary Component Testing Guide

## Overview

This document describes the comprehensive test suite for the `ErrorBoundary` component located in `frontend/src/components/__tests__/ErrorBoundary.test.tsx`.

## Test Coverage

### 1. Normal Rendering (2 tests)

Tests that verify the ErrorBoundary renders children correctly when no errors occur:

- ✅ Renders children when no error occurs
- ✅ Renders multiple children without error

### 2. Error Catching (4 tests)

Tests that verify the ErrorBoundary properly catches and handles errors:

- ✅ Catches errors thrown by children
- ✅ Logs errors to the logger
- ✅ Calls onError callback when provided
- ✅ Doesn't call onError callback if not provided

### 3. Fallback UI (6 tests)

Tests that verify the error UI is rendered correctly:

- ✅ Renders default error UI when error occurs
- ✅ Displays error details in development mode
- ✅ Hides error details in production mode
- ✅ Renders custom fallback when provided
- ✅ Displays stack trace in development mode
- ✅ Stack trace is expandable/collapsible

### 4. Error Recovery (3 tests)

Tests that verify users can recover from errors:

- ✅ Resets error state when Try Again button is clicked
- ✅ Has Go Home link that navigates to /
- ✅ Displays support message for persistent errors

### 5. Error State Management (2 tests)

Tests that verify error state is properly managed:

- ✅ Stores error and errorInfo in state
- ✅ Handles multiple errors sequentially

### 6. UI Elements (4 tests)

Tests that verify all UI elements are present and styled:

- ✅ Renders Try Again button
- ✅ Renders alert icon
- ✅ Has proper styling classes
- ✅ Supports dark mode classes

### 7. Edge Cases (5 tests)

Tests that verify the component handles edge cases gracefully:

- ✅ Handles errors with no message
- ✅ Handles null children gracefully
- ✅ Handles empty children
- ✅ Handles rapid error resets
- ✅ Handles errors during render

### 8. Accessibility (4 tests)

Tests that verify the component is accessible:

- ✅ Has proper heading hierarchy (h1)
- ✅ Has accessible buttons with proper labels
- ✅ Has accessible links with proper href
- ✅ Has proper contrast for error message

### 9. Integration Scenarios (3 tests)

Tests that verify the component works in complex scenarios:

- ✅ Works with nested error boundaries
- ✅ Works with conditional rendering
- ✅ Works with async components

## Total Test Count: 33 tests

## Running the Tests

### Run all ErrorBoundary tests:

```bash
npm run test -- ErrorBoundary --run
```

### Run tests in watch mode:

```bash
npm run test -- ErrorBoundary
```

### Run tests with UI:

```bash
npm run test:ui
```

### Run tests with coverage:

```bash
npm run test -- ErrorBoundary --coverage
```

## Test Scenarios Covered

### Error Scenarios

1. **Synchronous errors**: Errors thrown during render
2. **Error callbacks**: Custom error handling via onError prop
3. **Multiple errors**: Sequential error handling
4. **Nested boundaries**: Error boundaries within error boundaries

### UI Scenarios

1. **Default UI**: Standard error display
2. **Custom fallback**: Custom error UI via fallback prop
3. **Development mode**: Error details and stack traces
4. **Production mode**: Minimal error information
5. **Dark mode**: Proper styling in dark theme

### Recovery Scenarios

1. **Try Again button**: Reset error state
2. **Go Home link**: Navigate to home page
3. **Support message**: Help text for persistent errors

### Accessibility Scenarios

1. **Heading hierarchy**: Proper h1 for error title
2. **Button labels**: Clear button text
3. **Link targets**: Proper href attributes
4. **Color contrast**: WCAG compliant contrast ratios

## Mocked Dependencies

### Logger

```typescript
vi.mock("@/lib/logger", () => ({
  logger: {
    error: vi.fn(),
  },
}));
```

### Navigation

```typescript
vi.mock('@/i18n/navigation', () => ({
  Link: ({ href, children, ...props }: any) => (
    <a href={href} {...props}>
      {children}
    </a>
  ),
}));
```

## Test Utilities

### ThrowError Component

A test component that throws an error on render:

```typescript
const ThrowError = ({ shouldThrow = true }: { shouldThrow?: boolean }) => {
  if (shouldThrow) {
    throw new Error('Test error message');
  }
  return <div>No error</div>;
};
```

### ThrowErrorOnClick Component

A test component that throws an error when a button is clicked:

```typescript
const ThrowErrorOnClick = () => {
  const [shouldThrow, setShouldThrow] = React.useState(false);
  if (shouldThrow) {
    throw new Error('Error thrown on click');
  }
  return <button onClick={() => setShouldThrow(true)}>Throw Error</button>;
};
```

## Coverage Goals

The test suite aims for:

- **Line coverage**: 100% of ErrorBoundary.tsx
- **Branch coverage**: 100% of conditional logic
- **Function coverage**: 100% of methods
- **Statement coverage**: 100% of statements

## Key Testing Patterns

### 1. Error Boundary Testing

```typescript
render(
  <ErrorBoundary>
    <ThrowError />
  </ErrorBoundary>
);
expect(screen.getByText('Something went wrong')).toBeInTheDocument();
```

### 2. Callback Testing

```typescript
const onError = vi.fn();
render(
  <ErrorBoundary onError={onError}>
    <ThrowError />
  </ErrorBoundary>
);
expect(onError).toHaveBeenCalledWith(expect.any(Error), expect.any(Object));
```

### 3. Recovery Testing

```typescript
const tryAgainButton = screen.getByRole("button", { name: /Try Again/i });
fireEvent.click(tryAgainButton);
expect(screen.getByText("No error")).toBeInTheDocument();
```

### 4. Environment Testing

```typescript
const originalEnv = process.env.NODE_ENV;
process.env.NODE_ENV = "development";
// Test development-specific behavior
process.env.NODE_ENV = originalEnv;
```

## Known Limitations

1. **Async errors**: Error boundaries don't catch errors in async code
2. **Event handlers**: Errors in event handlers aren't caught by error boundaries
3. **Server-side rendering**: Error boundaries only work on the client side

## Future Enhancements

1. Add tests for error reporting to external services
2. Add tests for error analytics tracking
3. Add tests for error recovery with retry logic
4. Add tests for error boundary with Suspense
5. Add performance tests for error boundary rendering

## Maintenance

When updating the ErrorBoundary component:

1. Update corresponding tests
2. Ensure all tests pass
3. Maintain 100% coverage
4. Update this documentation
5. Run accessibility checks

## Related Files

- Component: `frontend/src/components/ErrorBoundary.tsx`
- Tests: `frontend/src/components/__tests__/ErrorBoundary.test.tsx`
- Setup: `frontend/src/__tests__/setup.ts`
- Config: `frontend/vitest.config.ts`
