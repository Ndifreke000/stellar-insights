import React from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { CorridorRecentFailures } from "../corridors/CorridorRecentFailures";
import * as corridorApi from "@/lib/api/corridors";

vi.mock("@/lib/api/corridors", () => ({
  getCorridorFailures: vi.fn(),
}));

describe("CorridorRecentFailures", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders failures table with result codes and details", async () => {
    vi.mocked(corridorApi.getCorridorFailures).mockResolvedValueOnce({
      corridor_key: "USDC:TEST->XLM",
      items: [
        {
          id: "fail-1",
          transaction_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
          timestamp: "2026-10-17T12:00:00Z",
          source_account: "GAEXAMPLEACCOUNT1",
          destination_account: "GBEXAMPLEACCOUNT2",
          amount: 1500,
          result_code: "op_underfunded",
          category: "insufficient_funds",
          error_message: "Not enough source balance to fulfill payment",
        },
      ],
      total: 1,
      page: 1,
      page_size: 10,
      total_pages: 1,
    });

    render(<CorridorRecentFailures corridorKey="USDC:TEST->XLM" />);

    expect(screen.getByText(/Recent Failed Payments/i)).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText("op_underfunded")).toBeInTheDocument();
      expect(screen.getByText("$1500.00")).toBeInTheDocument();
      expect(screen.getByText(/insufficient funds/i)).toBeInTheDocument();
      expect(screen.getByText(/Not enough source balance to fulfill payment/i)).toBeInTheDocument();
    });
  });

  it("renders empty state when there are zero failures", async () => {
    vi.mocked(corridorApi.getCorridorFailures).mockResolvedValueOnce({
      corridor_key: "USDC:TEST->XLM",
      items: [],
      total: 0,
      page: 1,
      page_size: 10,
      total_pages: 0,
    });

    render(<CorridorRecentFailures corridorKey="USDC:TEST->XLM" />);

    await waitFor(() => {
      expect(screen.getByText(/No Recent Payment Failures/i)).toBeInTheDocument();
      expect(screen.getByText(/All recent payments in this corridor were settled successfully/i)).toBeInTheDocument();
    });
  });

  it("renders error state when fetch fails", async () => {
    vi.mocked(corridorApi.getCorridorFailures).mockRejectedValueOnce(
      new Error("Network connection error")
    );

    render(<CorridorRecentFailures corridorKey="USDC:TEST->XLM" />);

    await waitFor(() => {
      expect(screen.getByText(/Network connection error/i)).toBeInTheDocument();
      expect(screen.getByText(/Retry/i)).toBeInTheDocument();
    });
  });
});
