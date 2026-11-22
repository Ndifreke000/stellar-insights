/**
 * Cursor-paginated API client for Stellar transactions and payments (Issue #2835).
 */
import { api } from "./api";
import { appendPageParams, type PaginatedResponse } from "./pagination";

export interface StreamingTransaction {
  id: string;
  paging_token: string;
  transaction_hash: string;
  source_account: string;
  from: string;
  to: string;
  amount: string;
  asset_type: string;
  asset_code?: string | null;
  asset_issuer?: string | null;
  operation_type?: string | null;
  transaction_successful?: boolean | null;
  created_at: string;
}

export interface TransactionFilters {
  limit?: number;
  /** Opaque cursor from `pagination.next_cursor`. */
  cursor?: string;
  /** Filter by asset code (e.g. `USDC`, `XLM`). */
  asset_code?: string;
  /** Filter by settlement outcome. */
  status?: "successful" | "failed" | "";
  /** Free-text search across tx hash, source, or destination. */
  search?: string;
  /** Server-driven sort direction (`desc` newest-first by default). */
  sort_order?: "asc" | "desc";
}

export interface PendingMultisigTransaction {
  id: string;
  source_account: string;
  xdr: string;
  required_signatures: number;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface PendingTransactionsPage {
  data: PendingMultisigTransaction[];
  next_cursor: string | null;
}

/**
 * Fetch one cursor-paginated page of Stellar network payment transactions.
 */
export async function getStreamingTransactions(
  filters: TransactionFilters = {},
): Promise<PaginatedResponse<StreamingTransaction>> {
  const params = appendPageParams(new URLSearchParams(), {
    limit: filters.limit,
    cursor: filters.cursor,
  });

  if (filters.asset_code?.trim()) {
    params.set("asset_code", filters.asset_code.trim());
  }
  if (filters.status) {
    params.set("status", filters.status);
  }
  if (filters.search?.trim()) {
    params.set("search", filters.search.trim());
  }
  if (filters.sort_order) {
    params.set("sort_order", filters.sort_order);
  }

  const query = params.toString();
  return api.get<PaginatedResponse<StreamingTransaction>>(
    query ? `/rpc/payments?${query}` : "/rpc/payments",
  );
}

/**
 * Fetch one cursor-paginated page of pending multi-signature transactions
 * for the signed-in SEP-10 wallet.
 */
export async function getPendingTransactions(
  params: { cursor?: string; limit?: number } = {},
  authToken?: string,
): Promise<PendingTransactionsPage> {
  const qs = appendPageParams(new URLSearchParams(), params).toString();
  return api.get<PendingTransactionsPage>(
    qs ? `/transactions?${qs}` : "/transactions",
    authToken
      ? { headers: { Authorization: `Bearer ${authToken}` } }
      : undefined,
  );
}
