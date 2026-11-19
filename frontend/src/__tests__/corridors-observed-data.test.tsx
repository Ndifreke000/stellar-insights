import type { ReactElement, ReactNode } from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { renderWithProviders } from "@/test-utils/render";
import { UserPreferencesProvider } from "@/contexts/UserPreferencesContext";
import { ObservedDataNotice } from "@/components/corridors/ObservedDataError";

vi.mock("next/navigation", () => ({
  useSearchParams: () => new URLSearchParams(),
  useParams: () => ({ pair: "USDC-NGN" }),
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
  usePathname: () => "/en/corridors",
}));

vi.mock("@/i18n/navigation", () => ({
  Link: ({
    href,
    children,
    ...props
  }: {
    href: string;
    children: ReactNode;
  }) => (
    <a href={href} {...props}>
      {children}
    </a>
  ),
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
  usePathname: () => "/corridors",
  redirect: vi.fn(),
}));

const useCorridors = vi.hoisted(() => vi.fn());
const getCorridors = vi.hoisted(() => vi.fn());

vi.mock("next/dynamic", () => ({
  default: () =>
    function DynamicStub() {
      return null;
    },
}));

vi.mock("@/lib/react-query/queries", () => ({
  useCorridors: (...args: unknown[]) => useCorridors(...args),
}));

vi.mock("@/lib/api/corridors", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api/corridors")>(
    "@/lib/api/corridors",
  );
  return {
    ...actual,
    getCorridors: (...args: unknown[]) => getCorridors(...args),
  };
});

import CorridorsPage from "@/app/[locale]/corridors/page";
import ForecastingPage from "@/app/[locale]/corridors/forecasting/page";

const failedQuery = {
  data: undefined,
  isError: true,
  isPending: false,
  isFetching: false,
  refetch: vi.fn(),
};

function renderPage(ui: ReactElement) {
  return renderWithProviders(<UserPreferencesProvider>{ui}</UserPreferencesProvider>);
}

describe("ObservedDataNotice", () => {
  it("offers retry and a link to the status page", () => {
    const onRetry = vi.fn();
    render(<ObservedDataNotice message="Corridor data is unavailable." onRetry={onRetry} />);

    expect(screen.getByRole("alert")).toHaveTextContent("Corridor data is unavailable.");
    screen.getByRole("button", { name: "Retry" }).click();
    expect(onRetry).toHaveBeenCalledOnce();
    expect(screen.getByRole("link", { name: "Status page" })).toHaveAttribute(
      "href",
      expect.stringContaining("/status"),
    );
  });
});

describe("corridor pages when the API fails", () => {
  beforeEach(() => {
    useCorridors.mockReset();
    getCorridors.mockReset();
  });

  it("shows an error on the corridor list and does not render mock route metrics", () => {
    useCorridors.mockReturnValue(failedQuery);

    renderPage(<CorridorsPage />);

    expect(screen.getByRole("alert")).toHaveTextContent("Corridor data is unavailable.");
    expect(screen.getByRole("button", { name: "Retry" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Status page" })).toBeInTheDocument();
    expect(screen.queryByText("USDC-EUR")).not.toBeInTheDocument();
    expect(screen.queryByText("97.8%")).not.toBeInTheDocument();
  });

  it("keeps loaded corridors visible and warns when a later refresh fails", () => {
    useCorridors.mockReturnValue({
      ...failedQuery,
      data: {
        data: [
          {
            id: "ZZZ-QQQ",
            source_asset: "ZZZ",
            destination_asset: "QQQ",
            success_rate: 12.5,
            total_attempts: 4,
            successful_payments: 1,
            failed_payments: 3,
            average_latency_ms: 10,
            median_latency_ms: 10,
            p95_latency_ms: 10,
            p99_latency_ms: 10,
            liquidity_depth_usd: 20,
            liquidity_volume_24h_usd: 5,
            liquidity_trend: "stable",
            average_slippage_bps: 1,
            health_score: 40,
            last_updated: "2026-10-19T00:00:00Z",
          },
        ],
      },
    });

    renderPage(<CorridorsPage />);

    expect(screen.getByRole("status")).toHaveTextContent("last observed results");
    expect(screen.getByRole("heading", { name: /ZZZ/ })).toBeInTheDocument();
    expect(screen.queryByText("USDC-EUR")).not.toBeInTheDocument();
  });

  it("shows an error on forecasting and does not render mock corridors", async () => {
    getCorridors.mockRejectedValue(new Error("backend unavailable"));

    renderPage(<ForecastingPage />);

    expect(await screen.findByRole("alert")).toHaveTextContent("Corridor forecasts are unavailable.");
    expect(screen.queryByText("USDC-EUR")).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Status page" })).toBeInTheDocument();
  });
});
