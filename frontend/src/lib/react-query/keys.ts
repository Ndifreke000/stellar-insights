/**
 * Query key factory — the single source of truth for React Query cache keys.
 *
 * Keys are hierarchical so a whole resource can be invalidated at once:
 *   invalidateQueries({ queryKey: queryKeys.corridors })   // every corridor query
 *   invalidateQueries({ queryKey: queryKeys.corridorLists }) // only list queries
 *
 * Never inline key arrays in components; add a factory here instead so
 * invalidation stays consistent. See docs/FRONTEND_STATE_MANAGEMENT.md.
 */
import type { CorridorFilters } from "@/lib/api/corridors";
import type { ListAnchorsParams } from "@/lib/api/anchor";

export const queryKeys = {
  // Anchors
  anchors: ["anchors"] as const,
  anchorLists: ["anchors", "list"] as const,
  anchorList: (params: ListAnchorsParams = {}) =>
    ["anchors", "list", params] as const,
  anchorDetail: (address: string) => ["anchors", "detail", address] as const,
  anchor: (id: string) => ["anchors", id] as const,
  anchorAssets: (id: string) => ["anchors", id, "assets"] as const,

  // Corridors
  corridors: ["corridors"] as const,
  corridorLists: ["corridors", "list"] as const,
  corridorList: (filters: CorridorFilters = {}) =>
    ["corridors", "list", filters] as const,
  corridorDetail: (id: string) => ["corridors", "detail", id] as const,
  corridor: (id: string) => ["corridors", id] as const,
  corridorMetrics: (id: string) => ["corridors", id, "metrics"] as const,

  // Governance
  governance: ["governance"] as const,
  proposals: (status?: string) =>
    ["governance", "proposals", status ?? "all"] as const,

  // SEP-24
  sep24Anchors: ["sep24", "anchors"] as const,
  sep24Info: (server: string) => ["sep24", "info", server] as const,
  sep24Transactions: (server: string) =>
    ["sep24", "transactions", server] as const,

  // SEP-31
  sep31Anchors: ["sep31", "anchors"] as const,
  sep31Info: (server: string) => ["sep31", "info", server] as const,
  sep31Transactions: (server: string) =>
    ["sep31", "transactions", server] as const,

  // RPC
  rpcHealth: ["rpc", "health"] as const,
  rpcLedger: ["rpc", "ledger"] as const,
  rpcPayments: (account?: string) => ["rpc", "payments", account] as const,
  rpcTrades: ["rpc", "trades"] as const,
  rpcOrderbook: ["rpc", "orderbook"] as const,

  // Jobs
  jobStatus: ["jobs", "status"] as const,
  jobHealth: ["jobs", "health"] as const,
  jobMetrics: ["jobs", "metrics"] as const,

  // Cache stats
  cacheStats: ["cache", "stats"] as const,

  // Price feeds
  priceFeeds: ["price-feeds"] as const,
  priceFeed: (id: string) => ["price-feeds", id] as const,
} as const;
