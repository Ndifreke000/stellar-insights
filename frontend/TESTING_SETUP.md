# Testing Setup

This document explains the testing infrastructure added for error boundaries.

## Installation

Install the test dependencies:

```bash
cd frontend
npm install
```

This will install:
- `jest` - Test runner
- `@testing-library/react` - React testing utilities
- `@testing-library/jest-dom` - Custom Jest matchers
- `@testing-library/user-event` - User interaction simulation
- `jest-environment-jsdom` - DOM environment for tests
- `@types/jest` - TypeScript types for Jest

## Running Tests

```bash
# Run all tests once
npm test

# Run tests in watch mode (re-runs on file changes)
npm run test:watch

# Run tests with coverage report
npm run test:coverage
```

## Test Files

Tests are located in the `__tests__/` directory:
- `ChartErrorBoundary.test.tsx` - Tests for chart error boundary
- `WebSocketErrorBoundary.test.tsx` - Tests for WebSocket error boundary

## Configuration Files

- `jest.config.js` - Jest configuration with Next.js integration
- `jest.setup.js` - Test setup file (imports jest-dom matchers)

## What's Tested

### ChartErrorBoundary
✅ Renders children when no error occurs
✅ Catches errors and displays fallback UI
✅ Displays custom fallback when provided
✅ Calls onError callback with error details
✅ Resets error state on retry button click
✅ Shows error details in development mode
✅ Hides error details in production mode

### WebSocketErrorBoundary
✅ Renders children when no error occurs
✅ Catches errors and displays fallback UI
✅ Displays custom fallback when provided
✅ Calls onError callback with error details
✅ Resets error state on reconnect button click
✅ Shows error details in development mode
✅ Hides error details in production mode

## CI Integration

To add tests to the CI pipeline, update `.github/workflows/frontend.yml`:

```yaml
- name: Run tests
  working-directory: ./frontend
  run: npm test

- name: Run tests with coverage
  working-directory: ./frontend
  run: npm run test:coverage
```

## Writing New Tests

When adding new error boundaries or components, follow this pattern:

```tsx
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';
import { YourComponent } from '@/components/YourComponent';

describe('YourComponent', () => {
  it('should render correctly', () => {
    render(<YourComponent />);
    expect(screen.getByText('Expected Text')).toBeInTheDocument();
  });

  it('should handle user interaction', () => {
    render(<YourComponent />);
    const button = screen.getByRole('button');
    fireEvent.click(button);
    // Assert expected behavior
  });
});
```

## Troubleshooting

### Tests fail with module resolution errors
- Ensure `jest.config.js` has correct `moduleNameMapper` for path aliases
- Check that `tsconfig.json` paths match Jest configuration

### Tests fail with "Cannot find module '@testing-library/jest-dom'"
- Run `npm install` to ensure all dependencies are installed
- Check that `jest.setup.js` is properly configured in `jest.config.js`

### Tests timeout
- Increase timeout in test: `it('test name', async () => { /* ... */ }, 10000)`
- Or set global timeout in `jest.config.js`: `testTimeout: 10000`

## Next Steps

1. ✅ Install dependencies: `npm install`
2. ✅ Run tests: `npm test`
3. ⏳ Add tests to CI pipeline
4. ⏳ Write tests for wrapped components
5. ⏳ Add integration tests for error scenarios
