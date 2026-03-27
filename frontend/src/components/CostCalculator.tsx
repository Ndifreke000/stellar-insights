"use client";

import React, { useMemo, useState } from "react";
import { Calculator, Loader2, Route, TrendingUp } from "lucide-react";

type RouteKey = "stellar_dex" | "anchor_direct" | "liquidity_pool";

interface RouteCostBreakdown {
  exchange_rate_mid: number;
  effective_rate: number;
  spread_bps: number;
  slippage_bps: number;
  spread_cost_source: number;
  service_fee_source: number;
  network_fee_source: number;
  slippage_cost_source: number;
  total_fees_source: number;
  total_fees_destination: number;
  estimated_destination_amount: number;
  destination_shortfall?: number;
  additional_source_required?: number;
}

interface RouteEstimate {
  route: RouteKey;
  route_name: string;
  breakdown: RouteCostBreakdown;
}

interface CostCalculationResponse {
  source_currency: string;
  destination_currency: string;
  source_amount: number;
  destination_amount?: number;
  source_usd_rate: number;
  destination_usd_rate: number;
  mid_market_rate: number;
  best_route: RouteEstimate;
  routes: RouteEstimate[];
  timestamp: string;
}

const DEFAULT_API_BASE =
  process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080/api";

const CURRENCIES = [
  "USD",
  "USDC",
  "EUR",
  "EURC",
  "NGN",
  "KES",
  "GHS",
  "PHP",
  "INR",
  "XLM",
];

const ROUTE_OPTIONS: Array<{ value: RouteKey; label: string }> = [
  { value: "stellar_dex", label: "Stellar DEX" },
  { value: "anchor_direct", label: "Anchor Direct" },
  { value: "liquidity_pool", label: "Liquidity Pool" },
];

function formatAmount(value: number, digits = 2): string {
  if (!Number.isFinite(value)) return "-";
  return value.toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: digits,
  });
}

export function CostCalculator() {
  const [sourceCurrency, setSourceCurrency] = useState("USDC");
  const [destinationCurrency, setDestinationCurrency] = useState("NGN");
  const [sourceAmount, setSourceAmount] = useState("1000");
  const [destinationAmount, setDestinationAmount] = useState("");
  const [selectedRoutes, setSelectedRoutes] = useState<RouteKey[]>([
    "stellar_dex",
    "anchor_direct",
    "liquidity_pool",
  ]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<CostCalculationResponse | null>(null);

  const canSubmit = useMemo(() => {
    const parsed = Number(sourceAmount);
    return Number.isFinite(parsed) && parsed > 0 && selectedRoutes.length > 0;
  }, [sourceAmount, selectedRoutes]);

  async function handleCalculate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!canSubmit) {
      setError("Enter a valid amount and select at least one route.");
      return;
    }

    setLoading(true);
    setError(null);

    const body = {
      source_currency: sourceCurrency,
      destination_currency: destinationCurrency,
      source_amount: Number(sourceAmount),
      destination_amount: destinationAmount ? Number(destinationAmount) : undefined,
      routes: selectedRoutes,
    };

    try {
      const response = await fetch(`${DEFAULT_API_BASE}/cost-calculator/estimate`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(body),
      });

      const payload = await response.json().catch(() => null);

      if (!response.ok) {
        const message = payload?.error || "Failed to calculate payment costs.";
        throw new Error(message);
      }

      setResult(payload as CostCalculationResponse);
    } catch (requestError) {
      const message =
        requestError instanceof Error
          ? requestError.message
          : "Failed to calculate payment costs.";
      setError(message);
      setResult(null);
    } finally {
      setLoading(false);
    }
  }

  function toggleRoute(route: RouteKey) {
    setSelectedRoutes((previous) => {
      if (previous.includes(route)) {
        return previous.filter((value) => value !== route);
      }
      return [...previous, route];
    });
  }

  return (
    <div className="space-y-6">
      <form
        onSubmit={handleCalculate}
        className="glass rounded-2xl border border-border/60 p-6 space-y-6"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <label className="space-y-2">
            <span className="text-xs font-mono uppercase tracking-[0.2em] text-muted-foreground">
              Source Currency
            </span>
            <select
              className="w-full rounded-xl border border-border bg-background/60 p-3 text-sm"
              value={sourceCurrency}
              onChange={(event) => setSourceCurrency(event.target.value)}
            >
              {CURRENCIES.map((currency) => (
                <option key={currency} value={currency}>
                  {currency}
                </option>
              ))}
            </select>
          </label>

          <label className="space-y-2">
            <span className="text-xs font-mono uppercase tracking-[0.2em] text-muted-foreground">
              Destination Currency
            </span>
            <select
              className="w-full rounded-xl border border-border bg-background/60 p-3 text-sm"
              value={destinationCurrency}
              onChange={(event) => setDestinationCurrency(event.target.value)}
            >
              {CURRENCIES.map((currency) => (
                <option key={currency} value={currency}>
                  {currency}
                </option>
              ))}
            </select>
          </label>

          <label className="space-y-2">
            <span className="text-xs font-mono uppercase tracking-[0.2em] text-muted-foreground">
              Source Amount
            </span>
            <input
              type="number"
              step="0.01"
              min="0"
              value={sourceAmount}
              onChange={(event) => setSourceAmount(event.target.value)}
              className="w-full rounded-xl border border-border bg-background/60 p-3 text-sm"
              placeholder="1000"
            />
          </label>

          <label className="space-y-2">
            <span className="text-xs font-mono uppercase tracking-[0.2em] text-muted-foreground">
              Target Destination Amount (Optional)
            </span>
            <input
              type="number"
              step="0.01"
              min="0"
              value={destinationAmount}
              onChange={(event) => setDestinationAmount(event.target.value)}
              className="w-full rounded-xl border border-border bg-background/60 p-3 text-sm"
              placeholder="Leave empty for estimate only"
            />
          </label>
        </div>

        <div className="space-y-3">
          <p className="text-xs font-mono uppercase tracking-[0.2em] text-muted-foreground">
            Compare Routes
          </p>
          <div className="flex flex-wrap gap-3">
            {ROUTE_OPTIONS.map((option) => (
              <label
                key={option.value}
                className="flex items-center gap-2 rounded-xl border border-border px-3 py-2 text-sm"
              >
                <input
                  type="checkbox"
                  checked={selectedRoutes.includes(option.value)}
                  onChange={() => toggleRoute(option.value)}
                />
                <span>{option.label}</span>
              </label>
            ))}
          </div>
        </div>

        <button
          type="submit"
          disabled={!canSubmit || loading}
          className="inline-flex items-center gap-2 rounded-xl bg-accent px-5 py-3 text-sm font-semibold text-white disabled:opacity-60"
        >
          {loading ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Calculating...
            </>
          ) : (
            <>
              <Calculator className="h-4 w-4" />
              Calculate Total Cost
            </>
          )}
        </button>
      </form>

      {error ? (
        <div className="rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-200">
          {error}
        </div>
      ) : null}

      {result ? (
        <div className="space-y-4">
          <div className="glass rounded-2xl border border-border/60 p-5">
            <p className="text-xs font-mono uppercase tracking-[0.2em] text-muted-foreground mb-2">
              Summary
            </p>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
              <div>
                <p className="text-muted-foreground">Mid-market rate</p>
                <p className="font-semibold">
                  1 {result.source_currency} = {formatAmount(result.mid_market_rate, 6)} {result.destination_currency}
                </p>
              </div>
              <div>
                <p className="text-muted-foreground">Best route</p>
                <p className="font-semibold text-accent">{result.best_route.route_name}</p>
              </div>
              <div>
                <p className="text-muted-foreground">Estimated destination</p>
                <p className="font-semibold">
                  {formatAmount(result.best_route.breakdown.estimated_destination_amount, 4)} {result.destination_currency}
                </p>
              </div>
            </div>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            {result.routes.map((route) => (
              <article
                key={route.route}
                className="rounded-2xl border border-border/60 bg-background/40 p-5 space-y-3"
              >
                <div className="flex items-center justify-between">
                  <h3 className="font-semibold flex items-center gap-2">
                    <Route className="h-4 w-4 text-accent" />
                    {route.route_name}
                  </h3>
                  {route.route === result.best_route.route ? (
                    <span className="text-[10px] px-2 py-1 rounded-full bg-accent/20 text-accent font-semibold uppercase tracking-[0.15em]">
                      Best
                    </span>
                  ) : null}
                </div>

                <div className="space-y-2 text-sm">
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Estimated destination</span>
                    <span className="font-semibold">
                      {formatAmount(route.breakdown.estimated_destination_amount, 4)} {result.destination_currency}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Total fees ({result.source_currency})</span>
                    <span>{formatAmount(route.breakdown.total_fees_source, 4)}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Service fee</span>
                    <span>{formatAmount(route.breakdown.service_fee_source, 4)}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Network fee</span>
                    <span>{formatAmount(route.breakdown.network_fee_source, 4)}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Spread cost</span>
                    <span>{formatAmount(route.breakdown.spread_cost_source, 4)}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Slippage cost</span>
                    <span>{formatAmount(route.breakdown.slippage_cost_source, 4)}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Effective rate</span>
                    <span className="inline-flex items-center gap-1">
                      <TrendingUp className="h-3 w-3 text-accent" />
                      {formatAmount(route.breakdown.effective_rate, 6)}
                    </span>
                  </div>
                </div>

                {route.breakdown.destination_shortfall ? (
                  <p className="rounded-lg border border-amber-500/40 bg-amber-500/10 p-2 text-xs text-amber-300">
                    Shortfall: {formatAmount(route.breakdown.destination_shortfall, 4)} {result.destination_currency}
                    {route.breakdown.additional_source_required
                      ? ` (additional ${formatAmount(route.breakdown.additional_source_required, 4)} ${result.source_currency} needed)`
                      : ""}
                  </p>
                ) : null}
              </article>
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}
