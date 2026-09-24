import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ChartErrorBoundary } from "../ChartErrorBoundary";
import { logger } from "@/lib/logger";

vi.mock("@/lib/logger", () => ({
  logger: {
    error: vi.fn(),
  },
}));

const ThrowError = ({ shouldThrow = true }: { shouldThrow?: boolean }) => {
  if (shouldThrow) {
    throw new Error("Chart render error");
  }
  return <div>Chart content</div>;
};

describe("ChartErrorBoundary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders children when no error occurs", () => {
    render(
      <ChartErrorBoundary>
        <div>Chart content</div>
      </ChartErrorBoundary>,
    );

    expect(screen.getByText("Chart content")).toBeInTheDocument();
  });

  it("renders the default fallback UI with the chart title when a child throws", () => {
    render(
      <ChartErrorBoundary chartTitle="Corridor volume">
        <ThrowError />
      </ChartErrorBoundary>,
    );

    expect(screen.getByText("Corridor volume — Failed to load")).toBeInTheDocument();
  });

  it("falls back to a generic title when chartTitle is not provided", () => {
    render(
      <ChartErrorBoundary>
        <ThrowError />
      </ChartErrorBoundary>,
    );

    expect(screen.getByText("Chart — Failed to load")).toBeInTheDocument();
  });

  it("logs the error via the shared logger", () => {
    render(
      <ChartErrorBoundary>
        <ThrowError />
      </ChartErrorBoundary>,
    );

    expect(logger.error).toHaveBeenCalledWith(
      "ChartErrorBoundary caught an error:",
      expect.any(Error),
      expect.objectContaining({ componentStack: expect.any(String) }),
    );
  });

  it("calls the onError callback when provided", () => {
    const onError = vi.fn();

    render(
      <ChartErrorBoundary onError={onError}>
        <ThrowError />
      </ChartErrorBoundary>,
    );

    expect(onError).toHaveBeenCalledWith(
      expect.any(Error),
      expect.objectContaining({ componentStack: expect.any(String) }),
    );
  });

  it("renders a custom fallback when provided instead of the default UI", () => {
    render(
      <ChartErrorBoundary fallback={<div>Custom chart fallback</div>}>
        <ThrowError />
      </ChartErrorBoundary>,
    );

    expect(screen.getByText("Custom chart fallback")).toBeInTheDocument();
    expect(screen.queryByText("Chart — Failed to load")).not.toBeInTheDocument();
  });

  it("recovers and renders children again after clicking Retry", () => {
    const { rerender } = render(
      <ChartErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ChartErrorBoundary>,
    );

    expect(screen.getByText("Chart — Failed to load")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Retry/i }));

    rerender(
      <ChartErrorBoundary>
        <ThrowError shouldThrow={false} />
      </ChartErrorBoundary>,
    );

    expect(screen.getByText("Chart content")).toBeInTheDocument();
  });
});
