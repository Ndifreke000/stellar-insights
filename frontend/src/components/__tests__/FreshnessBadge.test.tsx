import React from "react";
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { FreshnessBadge } from "@/components/ui/FreshnessBadge";

describe("FreshnessBadge", () => {
  it("renders with custom timestamp text", () => {
    render(<FreshnessBadge timestamp="2026-10-17 14:30:00 UTC" />);
    expect(screen.getByText(/2026-10-17 14:30:00 UTC/i)).toBeInTheDocument();
    expect(screen.getByText(/data as of/i)).toBeInTheDocument();
  });

  it("renders with Date object", () => {
    const date = new Date("2026-10-17T12:00:00Z");
    render(<FreshnessBadge timestamp={date} />);
    expect(screen.getByText(/data as of/i)).toBeInTheDocument();
  });

  it("applies custom label", () => {
    render(<FreshnessBadge timestamp="Just now" label="Updated at" />);
    expect(screen.getByText(/Updated at/i)).toBeInTheDocument();
    expect(screen.getByText(/Just now/i)).toBeInTheDocument();
  });

  it("renders tooltip with ISO string", () => {
    render(<FreshnessBadge timestamp="2026-10-17T10:00:00Z" />);
    const badge = screen.getByTitle(/Data snapshot timestamp: 2026-10-17T10:00:00Z/i);
    expect(badge).toBeInTheDocument();
  });
});
