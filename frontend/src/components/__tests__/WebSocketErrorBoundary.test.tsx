import React from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { WebSocketErrorBoundary } from "../WebSocketErrorBoundary";
import { logger } from "@/lib/logger";

vi.mock("@/lib/logger", () => ({
  logger: {
    error: vi.fn(),
  },
}));

const ThrowError = ({ shouldThrow = true }: { shouldThrow?: boolean }) => {
  if (shouldThrow) {
    throw new Error("Socket dropped");
  }
  return <div>Live data</div>;
};

describe("WebSocketErrorBoundary", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders children when no error occurs", () => {
    render(
      <WebSocketErrorBoundary>
        <div>Live data</div>
      </WebSocketErrorBoundary>,
    );

    expect(screen.getByText("Live data")).toBeInTheDocument();
  });

  it("shows the connection-issue fallback when a child throws", () => {
    render(
      <WebSocketErrorBoundary>
        <ThrowError />
      </WebSocketErrorBoundary>,
    );

    expect(screen.getByText("Connection Issue")).toBeInTheDocument();
    expect(
      screen.getByText(/Unable to establish real-time connection/),
    ).toBeInTheDocument();
  });

  it("logs the error and forwards it to onError", () => {
    const onError = vi.fn();

    render(
      <WebSocketErrorBoundary onError={onError}>
        <ThrowError />
      </WebSocketErrorBoundary>,
    );

    expect(logger.error).toHaveBeenCalledWith(
      "WebSocket component error:",
      expect.any(Error),
      expect.objectContaining({ componentStack: expect.any(String) }),
    );
    expect(onError).toHaveBeenCalledWith(expect.any(Error));
  });

  it("dismisses the fallback and renders nothing when the dismiss button is clicked", () => {
    render(
      <WebSocketErrorBoundary>
        <ThrowError />
      </WebSocketErrorBoundary>,
    );

    fireEvent.click(screen.getByRole("button", { name: /Dismiss error/i }));

    expect(screen.queryByText("Connection Issue")).not.toBeInTheDocument();
  });

  it("remounts children after clicking Retry Connection", () => {
    const { rerender } = render(
      <WebSocketErrorBoundary>
        <ThrowError shouldThrow={true} />
      </WebSocketErrorBoundary>,
    );

    expect(screen.getByText("Connection Issue")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Retry Connection/i }));

    rerender(
      <WebSocketErrorBoundary>
        <ThrowError shouldThrow={false} />
      </WebSocketErrorBoundary>,
    );

    expect(screen.getByText("Live data")).toBeInTheDocument();
  });
});
