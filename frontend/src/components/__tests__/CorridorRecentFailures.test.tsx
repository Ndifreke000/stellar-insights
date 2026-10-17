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
      failures: [
        {
          id: "fail-1",
          transaction_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
          source_account: "GAEXAMPLEACCOUNT1",
          destination_account: "GBEXAMPLEACCOUNT2",
          amount: "1500.00",
          asset_code: "USDC",
          result_code: "op_underfunded",
          result_message: "Not enough source balance to fulfill payment",
          failed_at: "2026-10-17T12:00:00Z",
        },
      ],
      total: 1,
      page: 1,
      page_size: 10,
    });

    render(<CorridorRecentFailures corridorKey="USDC:TEST->XLM" />);

    expect(screen.getByText(/Recent Failures & Result Codes/i)).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText("op_underfunded")).toBeInTheDocument();
      expect(screen.getByText(/1,500 USDC/i)).toBeInTheDocument();
      expect(screen.getByText(/Not enough source balance to fulfill payment/i)).toBeInTheDocument();
    });
  });

  it("renders empty state when there are zero failures", async () => {
    vi.mocked(corridorApi.getCorridorFailures).mockResolvedValueOnce({
      failures: [],
      total: 0,
      page: 1,
      page_size: 10,
    });

    render(<CorridorRecentFailures corridorKey="USDC:TEST->XLM" />);

    await waitFor(() => {
      expect(screen.getByText(/No Recent Failures/i)).toBeInTheDocument();
      expect(screen.getByText(/All recent transactions in this corridor settled successfully/i)).toBeInTheDocument();
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
