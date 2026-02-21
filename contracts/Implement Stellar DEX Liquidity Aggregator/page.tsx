// frontend/src/app/corridors/[pair]/page.tsx
//
// Replace (or merge into) your existing corridor detail page.
// The key additions are:
//   1. fetchOrderBook() / fetchLiquidityMetrics() helpers
//   2. <MarketDepthChart /> rendered with live data
//   3. Auto-refresh every 5 minutes

"use client";

import { useEffect, useState, useCallback } from "react";
import MarketDepthChart, {
  LiquidityMetrics,
} from "@/components/charts/MarketDepthChart";

// ─── Types ────────────────────────────────────────────────────────────────────

interface PriceLevel {
  price: number;
  amount: number;
}

interface OrderBook {
  bids: PriceLevel[];
  asks: PriceLevel[];
}

interface CorridorDetail {
  id: string;
  base_asset: string;
  counter_asset: string;
  volume_24h: number;
  liquidity?: LiquidityMetrics;
}

// ─── API helpers ──────────────────────────────────────────────────────────────

const API_BASE = process.env.NEXT_PUBLIC_API_URL ?? "";

async function fetchCorridorDetail(pair: string): Promise<CorridorDetail> {
  const resp = await fetch(`${API_BASE}/api/corridors/${encodeURIComponent(pair)}`);
  if (!resp.ok) throw new Error(`Failed to load corridor: ${resp.status}`);
  return resp.json();
}

async function fetchOrderBook(
  baseCode: string,
  baseIssuer: string,
  counterCode: string,
  counterIssuer: string,
  limit = 50
): Promise<OrderBook> {
  const params = new URLSearchParams({
    selling_asset_type: baseCode === "XLM" ? "native" : baseCode.length <= 4 ? "credit_alphanum4" : "credit_alphanum12",
    buying_asset_type:  counterCode === "XLM" ? "native" : counterCode.length <= 4 ? "credit_alphanum4" : "credit_alphanum12",
    limit: String(limit),
  });
  if (baseCode !== "XLM") {
    params.set("selling_asset_code",   baseCode);
    params.set("selling_asset_issuer", baseIssuer);
  }
  if (counterCode !== "XLM") {
    params.set("buying_asset_code",   counterCode);
    params.set("buying_asset_issuer", counterIssuer);
  }

  // Go through our backend proxy so we keep the cache warm
  const resp = await fetch(`${API_BASE}/api/order_book?${params}`);
  if (!resp.ok) throw new Error(`Order book fetch failed: ${resp.status}`);
  const raw = await resp.json();

  const parse = (levels: { price: string; amount: string }[]): PriceLevel[] =>
    levels.map((l) => ({ price: parseFloat(l.price), amount: parseFloat(l.amount) }));

  return { bids: parse(raw.bids ?? []), asks: parse(raw.asks ?? []) };
}

// ─── Page component ───────────────────────────────────────────────────────────

export default function CorridorDetailPage({
  params,
}: {
  params: { pair: string };
}) {
  const { pair } = params;

  const [corridor, setCorridor] = useState<CorridorDetail | null>(null);
  const [orderBook, setOrderBook] = useState<OrderBook | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Derive individual asset components from the pair slug (e.g. "USDC-XLM")
  const [basePart, counterPart] = pair.split("-");
  const [baseCode, baseIssuer = ""] = basePart.split(":");
  const [counterCode, counterIssuer = ""] = counterPart?.split(":") ?? [];

  const refresh = useCallback(async () => {
    try {
      const [detail, ob] = await Promise.all([
        fetchCorridorDetail(pair),
        fetchOrderBook(baseCode, baseIssuer, counterCode, counterIssuer),
      ]);
      setCorridor(detail);
      setOrderBook(ob);
      setError(null);
    } catch (e: any) {
      setError(e.message ?? "Unknown error");
    } finally {
      setLoading(false);
    }
  }, [pair, baseCode, baseIssuer, counterCode, counterIssuer]);

  // Initial load + 5-minute auto-refresh
  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 5 * 60 * 1000);
    return () => clearInterval(id);
  }, [refresh]);

  // ── Render ──────────────────────────────────────────────────────────────────

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen text-gray-400">
        Loading corridor data…
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center min-h-screen text-red-400">
        Error: {error}
      </div>
    );
  }

  return (
    <main className="max-w-4xl mx-auto px-4 py-8 space-y-8">
      {/* Title */}
      <div>
        <h1 className="text-2xl font-bold text-white">
          {baseCode} / {counterCode}
        </h1>
        <p className="text-gray-400 text-sm mt-1">
          24h Volume: {corridor?.volume_24h.toLocaleString()}
        </p>
      </div>

      {/* Liquidity summary cards */}
      {corridor?.liquidity && (
        <section className="grid grid-cols-2 sm:grid-cols-3 gap-4">
          <Card label="Mid Price"       value={corridor.liquidity.mid_price.toFixed(6)} />
          <Card label="Spread (bps)"    value={corridor.liquidity.spread_bps.toFixed(1)} />
          <Card label="Best Bid"        value={corridor.liquidity.best_bid.toFixed(6)} />
          <Card label="Best Ask"        value={corridor.liquidity.best_ask.toFixed(6)} />
          <Card label="1% Depth"        value={corridor.liquidity.depth_at_1_percent.toLocaleString()} />
          <Card label="5% Depth"        value={corridor.liquidity.depth_at_5_percent.toLocaleString()} />
        </section>
      )}

      {/* Market depth chart */}
      {orderBook && (
        <section>
          <h2 className="text-lg font-semibold text-white mb-3">Market Depth</h2>
          <MarketDepthChart
            orderBook={orderBook}
            metrics={corridor?.liquidity}
            baseCurrency={baseCode}
            counterCurrency={counterCode}
          />
        </section>
      )}

      {/* Refresh indicator */}
      <p className="text-xs text-gray-600 text-right">
        Auto-refreshes every 5 minutes ·{" "}
        <button onClick={refresh} className="underline hover:text-gray-400">
          Refresh now
        </button>
      </p>
    </main>
  );
}

function Card({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-gray-800 rounded-lg p-4">
      <p className="text-xs text-gray-400">{label}</p>
      <p className="text-base font-semibold text-white mt-1">{value}</p>
    </div>
  );
}
