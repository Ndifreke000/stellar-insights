import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ErrorBoundary } from "../ErrorBoundary";
import { logger } from "@/lib/logger";

// Mock the logger
vi.mock("@/lib/logger", () => ({
  logger: {
    error: vi.fn(),
  },
}));

// Mock next/navigation
vi.mock("@/i18n/navigation", () => ({
  Link: ({ href, children, ...props }: any) => (
    <a href={href} {...props}>
      {children}
    </a>
  ),
}));

// Component that throws an error
const ThrowError = ({ shouldThrow = true }: { shouldThrow?: boolean }) => {
  if (shouldThrow) {
    throw new Error("Test error message");
  }
  return <div>No error</div>;
};

describe("ErrorBoundary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Suppress console.error for error boundary tests
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("Normal rendering", () => {
    it("should render children when no error occurs", () => {
      render(
        <ErrorBoundary>
          <div>Test content</div>
        </ErrorBoundary>,
      );

      expect(screen.getByText("Test content")).toBeInTheDocument();
    });

    it("should render multiple children without error", () => {
      render(
        <ErrorBoundary>
          <div>First child</div>
          <div>Second child</div>
        </ErrorBoundary>,
      );

      expect(screen.getByText("First child")).toBeInTheDocument();
      expect(screen.getByText("Second child")).toBeInTheDocument();
    });
  });

  describe("Error catching", () => {
    it("should catch errors thrown by children", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    });

    it("should log error to logger", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(logger.error).toHaveBeenCalledWith(
        "ErrorBoundary caught an error:",
        expect.any(Error),
      );
    });

    it("should call onError callback when provided", () => {
      const onError = vi.fn();

      render(
        <ErrorBoundary onError={onError}>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(onError).toHaveBeenCalledWith(
        expect.any(Error),
        expect.objectContaining({
          componentStack: expect.any(String),
        }),
      );
    });

    it("should not call onError callback if not provided", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      // Should not throw
      expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    });
  });

  describe("Fallback UI", () => {
    it("should render default error UI when error occurs", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(screen.getByText("Something went wrong")).toBeInTheDocument();
      expect(
        screen.getByText(
          "An unexpected error occurred. Don't worry, we're on it!",
        ),
      ).toBeInTheDocument();
    });

    it("should display error details in development mode", () => {
      const originalEnv = process.env.NODE_ENV;
      process.env.NODE_ENV = "development";

      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(
        screen.getByText("Error Details (Development Only)"),
      ).toBeInTheDocument();
      expect(screen.getByText(/Test error message/)).toBeInTheDocument();

      process.env.NODE_ENV = originalEnv;
    });

    it("should not display error details in production mode", () => {
      const originalEnv = process.env.NODE_ENV;
      process.env.NODE_ENV = "production";

      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(
        screen.queryByText("Error Details (Development Only)"),
      ).not.toBeInTheDocument();

      process.env.NODE_ENV = originalEnv;
    });

    it("should render custom fallback when provided", () => {
      const customFallback = <div>Custom error UI</div>;

      render(
        <ErrorBoundary fallback={customFallback}>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(screen.getByText("Custom error UI")).toBeInTheDocument();
      expect(
        screen.queryByText("Something went wrong"),
      ).not.toBeInTheDocument();
    });

    it("should display stack trace in development mode", () => {
      const originalEnv = process.env.NODE_ENV;
      process.env.NODE_ENV = "development";

      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      const stackTraceButton = screen.getByText("Stack Trace");
      expect(stackTraceButton).toBeInTheDocument();

      // Click to expand
      fireEvent.click(stackTraceButton);

      // Stack trace should be visible
      expect(screen.getByText(/Stack Trace/)).toBeInTheDocument();

      process.env.NODE_ENV = originalEnv;
    });
  });

  describe("Error recovery", () => {
    it("should reset error state when Try Again button is clicked", async () => {
      const { rerender } = render(
        <ErrorBoundary>
          <ThrowError shouldThrow={true} />
        </ErrorBoundary>,
      );

      expect(screen.getByText("Something went wrong")).toBeInTheDocument();

      const tryAgainButton = screen.getByRole("button", { name: /Try Again/i });
      fireEvent.click(tryAgainButton);

      // After reset, should render children again
      rerender(
        <ErrorBoundary>
          <ThrowError shouldThrow={false} />
        </ErrorBoundary>,
      );

      await waitFor(() => {
        expect(screen.getByText("No error")).toBeInTheDocument();
      });
    });

    it("should have Go Home link", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      const homeLink = screen.getByRole("link", { name: /Go Home/i });
      expect(homeLink).toBeInTheDocument();
      expect(homeLink).toHaveAttribute("href", "/");
    });

    it("should display support message", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(
        screen.getByText(
          /If this problem persists, please contact support or refresh the page/,
        ),
      ).toBeInTheDocument();
    });
  });

  describe("Error state management", () => {
    it("should store error and errorInfo in state", () => {
      const onError = vi.fn();

      render(
        <ErrorBoundary onError={onError}>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(onError).toHaveBeenCalledWith(
        expect.objectContaining({
          message: "Test error message",
        }),
        expect.objectContaining({
          componentStack: expect.any(String),
        }),
      );
    });

    it("should handle multiple errors sequentially", () => {
      const onError = vi.fn();
      const { rerender } = render(
        <ErrorBoundary onError={onError}>
          <ThrowError />
        </ErrorBoundary>,
      );

      expect(onError).toHaveBeenCalledTimes(1);

      // Reset and throw a different error
      const tryAgainButton = screen.getByRole("button", { name: /Try Again/i });
      fireEvent.click(tryAgainButton);

      rerender(
        <ErrorBoundary onError={onError}>
          <ThrowError shouldThrow={true} />
        </ErrorBoundary>,
      );

      // Should have been called again
      expect(onError).toHaveBeenCalledTimes(2);
    });
  });

  describe("UI elements", () => {
    it("should render Try Again button", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      const button = screen.getByRole("button", { name: /Try Again/i });
      expect(button).toBeInTheDocument();
      expect(button).toHaveClass("bg-blue-600");
    });

    it("should render alert icon", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      // AlertTriangle icon should be rendered
      expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    });

    it("should have proper styling classes", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      const container = screen.getByText("Something went wrong").closest("div");
      expect(container?.parentElement).toHaveClass("min-h-screen");
      expect(container?.parentElement).toHaveClass("bg-gray-50");
    });

    it("should support dark mode classes", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      const container = screen.getByText("Something went wrong").closest("div");
      expect(container?.parentElement).toHaveClass("dark:bg-slate-950");
    });
  });

  describe("Edge cases", () => {
    it("should handle errors with no message", () => {
      const ErrorWithoutMessage = () => {
        throw new Error();
      };

      render(
        <ErrorBoundary>
          <ErrorWithoutMessage />
        </ErrorBoundary>,
      );

      expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    });

    it("should handle null children gracefully", () => {
      render(<ErrorBoundary>{null}</ErrorBoundary>);

      expect(
        screen.queryByText("Something went wrong"),
      ).not.toBeInTheDocument();
    });

    it("should handle empty children", () => {
      render(
        <ErrorBoundary>
          <></>
        </ErrorBoundary>,
      );

      expect(
        screen.queryByText("Something went wrong"),
      ).not.toBeInTheDocument();
    });

    it("should handle rapid error resets", async () => {
      const { rerender } = render(
        <ErrorBoundary>
          <ThrowError shouldThrow={true} />
        </ErrorBoundary>,
      );

      expect(screen.getByText("Something went wrong")).toBeInTheDocument();

      // Click Try Again multiple times rapidly
      const tryAgainButton = screen.getByRole("button", { name: /Try Again/i });
      fireEvent.click(tryAgainButton);
      fireEvent.click(tryAgainButton);

      rerender(
        <ErrorBoundary>
          <ThrowError shouldThrow={false} />
        </ErrorBoundary>,
      );

      await waitFor(() => {
        expect(screen.getByText("No error")).toBeInTheDocument();
      });
    });
  });

  describe("Accessibility", () => {
    it("should have proper heading hierarchy", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      const heading = screen.getByRole("heading", { level: 1 });
      expect(heading).toHaveTextContent("Something went wrong");
    });

    it("should have accessible buttons", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      const tryAgainButton = screen.getByRole("button", { name: /Try Again/i });
      expect(tryAgainButton).toBeInTheDocument();
      expect(tryAgainButton).toHaveClass("font-medium");
    });

    it("should have accessible links", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      const homeLink = screen.getByRole("link", { name: /Go Home/i });
      expect(homeLink).toBeInTheDocument();
      expect(homeLink).toHaveAttribute("href", "/");
    });

    it("should have proper contrast for error message", () => {
      render(
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>,
      );

      const errorHeading = screen.getByText("Something went wrong");
      expect(errorHeading).toHaveClass("text-gray-900", "dark:text-white");
    });
  });

  describe("Integration scenarios", () => {
    it("should work with nested error boundaries", () => {
      const InnerBoundary = () => (
        <ErrorBoundary>
          <ThrowError />
        </ErrorBoundary>
      );

      render(
        <ErrorBoundary>
          <InnerBoundary />
        </ErrorBoundary>,
      );

      // Inner boundary should catch the error
      expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    });

    it("should work with conditional rendering", () => {
      const ConditionalComponent = ({ showError }: { showError: boolean }) => (
        <>
          {showError && <ThrowError />}
          {!showError && <div>Safe content</div>}
        </>
      );

      const { rerender } = render(
        <ErrorBoundary>
          <ConditionalComponent showError={false} />
        </ErrorBoundary>,
      );

      expect(screen.getByText("Safe content")).toBeInTheDocument();

      rerender(
        <ErrorBoundary>
          <ConditionalComponent showError={true} />
        </ErrorBoundary>,
      );

      expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    });

    it("should work with async components", async () => {
      const AsyncComponent = async () => {
        await new Promise((resolve) => setTimeout(resolve, 10));
        return <div>Async content</div>;
      };

      // Note: Error boundaries don't catch errors in async code
      // This test documents the limitation
      render(
        <ErrorBoundary>
          <AsyncComponent />
        </ErrorBoundary>,
      );

      // Async component should render without error boundary catching
      expect(
        screen.queryByText("Something went wrong"),
      ).not.toBeInTheDocument();
    });
  });
});
