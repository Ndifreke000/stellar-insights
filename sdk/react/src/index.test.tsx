import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import type { StellarInsights } from "@stellar-insights/sdk";
import { StellarInsightsProvider } from "./StellarInsightsProvider.js";
import { CorridorReliabilityCard } from "./components/CorridorReliabilityCard.js";

function fakeClient(overrides: Partial<StellarInsights["corridors"]> = {}) {
  return {
    corridors: {
      list: vi.fn(),
      get: vi.fn(),
      ...overrides,
    },
  } as unknown as StellarInsights;
}

describe("CorridorReliabilityCard", () => {
  it("shows a loading state, then renders corridor data", async () => {
    const client = fakeClient({
      get: vi.fn().mockResolvedValue({
        source: "USDC",
        destination: "BRL",
        volume_usd: 1_250_000,
        success_rate: 0.9987,
        avg_latency_ms: 340,
        success_rate_history: [],
        latency_history: [],
        liquidity_history: [],
      }),
    });

    render(
      <StellarInsightsProvider client={client}>
        <CorridorReliabilityCard source="USDC" destination="BRL" />
      </StellarInsightsProvider>,
    );

    expect(screen.getByRole("status")).toHaveTextContent("Loading USDC/BRL");

    await waitFor(() => expect(screen.getByText("USDC/BRL")).toBeInTheDocument());
    expect(screen.getByText(/Success rate: 99.87%/)).toBeInTheDocument();
    expect(screen.getByText(/Avg latency: 340ms/)).toBeInTheDocument();
  });

  it("shows an error state when the request fails", async () => {
    const client = fakeClient({
      get: vi.fn().mockRejectedValue(new Error("network down")),
    });

    render(
      <StellarInsightsProvider client={client}>
        <CorridorReliabilityCard source="USDC" destination="BRL" />
      </StellarInsightsProvider>,
    );

    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("network down"));
  });
});
