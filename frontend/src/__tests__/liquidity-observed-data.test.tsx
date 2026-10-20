import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import { renderWithProviders } from "@/test-utils/render";

const fetchObservedLiquidity = vi.hoisted(() => vi.fn());

vi.mock("@/lib/liquidity-pool-api", () => ({
  fetchObservedLiquidity: (...args: unknown[]) => fetchObservedLiquidity(...args),
}));

vi.mock("@/components/charts/LiquidityChart", () => ({
  LiquidityChart: ({ data }: { data: { liquidity_usd: number }[] }) => (
    <div>liquidity-chart {data.map((point) => point.liquidity_usd).join(",")}</div>
  ),
}));

import LiquidityPage from "@/app/[locale]/liquidity/page";

const pool = {
  pool_id: "pool_real",
  pool_type: "constant_product",
  fee_bp: 30,
  total_trustlines: 1,
  total_shares: "1",
  reserve_a_asset_code: "USDC",
  reserve_a_asset_issuer: "GABC",
  reserve_a_amount: 10,
  reserve_b_asset_code: "XLM",
  reserve_b_asset_issuer: null,
  reserve_b_amount: 20,
  total_value_usd: 42,
  volume_24h_usd: 7,
  fees_earned_24h_usd: 0,
  apy: 1,
  impermanent_loss_pct: 0,
  trade_count_24h: 1,
  last_synced_at: "2026-10-19T12:00:00Z",
  created_at: "2026-10-19T12:00:00Z",
  updated_at: "2026-10-19T12:00:00Z",
};

describe("liquidity page observed state", () => {
  beforeEach(() => {
    fetchObservedLiquidity.mockReset();
  });

  it("shows an empty state when the API returns no pools", async () => {
    fetchObservedLiquidity.mockResolvedValue({
      points: [],
      stats: {
        total_pools: 0,
        total_value_locked_usd: 0,
        total_volume_24h_usd: 0,
        total_fees_24h_usd: 0,
        avg_apy: 0,
        avg_impermanent_loss: 0,
      },
      pools: [],
      partialWarning: null,
    });

    renderWithProviders(<LiquidityPage />);

    expect(await screen.findByText("No liquidity pools observed")).toBeInTheDocument();
    expect(screen.queryByText("liquidity-chart")).not.toBeInTheDocument();
    expect(screen.queryByText("$124.5M")).not.toBeInTheDocument();
  });

  it("renders the chart from observed pool points", async () => {
    fetchObservedLiquidity.mockResolvedValue({
      points: [
        {
          timestamp: "2026-10-19T12:00:00Z",
          liquidity_usd: 42,
          corridor_key: "USDC-XLM",
        },
      ],
      stats: {
        total_pools: 1,
        total_value_locked_usd: 42,
        total_volume_24h_usd: 7,
        total_fees_24h_usd: 0,
        avg_apy: 1,
        avg_impermanent_loss: 0,
      },
      pools: [pool],
      partialWarning: null,
    });

    renderWithProviders(<LiquidityPage />);

    expect(await screen.findByText("liquidity-chart 42")).toBeInTheDocument();
    expect(screen.queryByText("$124.5M")).not.toBeInTheDocument();
  });

  it("shows an error and no chart when the API fails", async () => {
    fetchObservedLiquidity.mockRejectedValue(new Error("pool api down"));

    renderWithProviders(<LiquidityPage />);

    expect(await screen.findByRole("alert")).toHaveTextContent("pool api down");
    expect(screen.queryByText("liquidity-chart")).not.toBeInTheDocument();
    expect(screen.queryByText("$124.5M")).not.toBeInTheDocument();
  });
});
