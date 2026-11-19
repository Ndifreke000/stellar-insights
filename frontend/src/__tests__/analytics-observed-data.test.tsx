import type { ReactNode } from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import { renderWithProviders } from "@/test-utils/render";

const getAnalyticsDashboard = vi.hoisted(() => vi.fn());

vi.mock("@/lib/analytics", () => ({
  getAnalyticsDashboard: (...args: unknown[]) => getAnalyticsDashboard(...args),
}));

vi.mock("@/components/analytics/MuxedAccountCard", () => ({
  MuxedAccountCard: () => null,
}));

vi.mock("@/components/OnChainVerification", () => ({
  OnChainVerification: () => null,
}));

vi.mock("@/components/PrintButton", () => ({
  PrintButton: () => null,
}));

vi.mock("@/i18n/navigation", () => ({
  Link: ({ href, children }: { href: string; children: ReactNode }) => (
    <a href={href}>{children}</a>
  ),
  useRouter: () => ({ push: vi.fn() }),
  usePathname: () => "/analytics",
}));

import AnalyticsPage from "@/app/[locale]/analytics/page";

describe("analytics page observed state", () => {
  beforeEach(() => {
    getAnalyticsDashboard.mockReset();
  });

  it("shows the unavailable banner when the dashboard request fails", async () => {
    getAnalyticsDashboard.mockRejectedValue(new Error("backend down"));

    renderWithProviders(<AnalyticsPage />);

    expect(await screen.findByRole("alert")).toHaveTextContent(/temporarily unavailable/i);
    expect(screen.queryByText(/Running on Local Cache/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Mock Data/i)).not.toBeInTheDocument();
    expect(screen.queryByText("$45K")).not.toBeInTheDocument();
  });

  it("renders observed volume when the dashboard has data", async () => {
    getAnalyticsDashboard.mockResolvedValue({
      data_available: true,
      stats: {
        volume_24h: 150,
        volume_growth: 0,
        avg_success_rate: 80,
        success_rate_growth: 0,
        active_corridors: 1,
        corridors_growth: 0,
      },
      time_series_data: [
        { time: "10-19 12:00", volume: 150, corridors: 1, anchors: 1 },
      ],
      series: [{ time: "10-19 12:00", volume: 150, corridors: 1, anchors: 1 }],
      corridor_performance: [
        { corridor: "USDC:ISS->NGN:ISS", success_rate: 80, volume: 150, health: 80 },
      ],
    });

    renderWithProviders(<AnalyticsPage />);

    await waitFor(() => {
      expect(screen.getByText("10-19 12:00")).toBeInTheDocument();
    });
    expect(screen.getByText("USDC:ISS->NGN:ISS")).toBeInTheDocument();
    expect(screen.queryByText(/Mock Data/i)).not.toBeInTheDocument();
  });
});
