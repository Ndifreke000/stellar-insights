import { signTransaction } from "@stellar/freighter-api";
import {
  Asset,
  BASE_FEE,
  Horizon,
  Memo,
  Networks,
  Operation,
  TransactionBuilder,
} from "@stellar/stellar-sdk";
import type { Invoice } from "@/lib/api-keys";

const HORIZON_URL: Record<"mainnet" | "testnet", string> = {
  mainnet: "https://horizon.stellar.org",
  testnet: "https://horizon-testnet.stellar.org",
};

function network(): "mainnet" | "testnet" {
  return process.env.NEXT_PUBLIC_STELLAR_NETWORK === "mainnet" ? "mainnet" : "testnet";
}

/**
 * Build the payment the invoice asks for, have Freighter sign it, submit it
 * and return the transaction hash. The memo and amount come straight from the
 * invoice so the backend's on-chain check matches.
 */
export async function payInvoiceWithFreighter(invoice: Invoice, payer: string): Promise<string> {
  const net = network();
  const passphrase = net === "mainnet" ? Networks.PUBLIC : Networks.TESTNET;
  const server = new Horizon.Server(HORIZON_URL[net]);

  const account = await server.loadAccount(payer);
  const transaction = new TransactionBuilder(account, {
    fee: BASE_FEE,
    networkPassphrase: passphrase,
  })
    .addOperation(
      Operation.payment({
        destination: invoice.destination,
        asset: new Asset(invoice.asset_code, invoice.asset_issuer),
        amount: invoice.amount_usdc,
      }),
    )
    .addMemo(Memo.text(invoice.memo))
    .setTimeout(180)
    .build();

  const signed = await signTransaction(transaction.toXDR(), {
    networkPassphrase: passphrase,
    address: payer,
  });
  if (signed.error || !signed.signedTxXdr) {
    throw new Error(signed.error?.message || "The wallet did not sign the payment.");
  }

  const result = await server.submitTransaction(
    TransactionBuilder.fromXDR(signed.signedTxXdr, passphrase),
  );
  return result.hash;
}

export type InvoiceStatus = "pending" | "paid" | "expired" | "failed";

export interface InvoiceStatusResult {
  status: InvoiceStatus;
  plan?: string;
  txHash?: string;
}

const POLL_INTERVAL_MS = 3000;
const POLL_TIMEOUT_MS = 120_000;

function apiBase(): string {
  return process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";
}

/**
 * Fetch the current status of an invoice from the backend. Returns `null` when
 * the backend responds with a non-OK status so callers can surface an error
 * state instead of fabricating a value.
 */
export async function fetchInvoiceStatus(invoiceId: string): Promise<InvoiceStatusResult | null> {
  const res = await fetch(`${apiBase()}/api/v1/billing/invoices/${encodeURIComponent(invoiceId)}`, {
    headers: { Accept: "application/json" },
  });
  if (!res.ok) return null;
  const data = (await res.json()) as Partial<InvoiceStatusResult>;
  if (!data.status) return null;
  return { status: data.status, plan: data.plan, txHash: data.txHash };
}

/**
 * Poll the invoice until the backend reports it as paid (plan upgraded) or the
 * invoice reaches a terminal state. Throws on timeout so the UI can show an
 * explicit error rather than hanging.
 */
export async function pollInvoiceUntilPaid(
  invoiceId: string,
  options: { intervalMs?: number; timeoutMs?: number; signal?: AbortSignal } = {},
): Promise<InvoiceStatusResult> {
  const intervalMs = options.intervalMs ?? POLL_INTERVAL_MS;
  const timeoutMs = options.timeoutMs ?? POLL_TIMEOUT_MS;
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    if (options.signal?.aborted) {
      throw new Error("Payment polling was cancelled.");
    }
    const current = await fetchInvoiceStatus(invoiceId);
    if (current) {
      if (current.status === "paid") return current;
      if (current.status === "expired" || current.status === "failed") {
        throw new Error(`Invoice ${current.status}.`);
      }
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
  throw new Error("Timed out waiting for the payment to confirm.");
}

/**
 * One-click Pro upgrade: build + sign the USDC payment with Freighter, submit
 * it, then poll the invoice until the backend upgrades the plan.
 */
export async function payProPlanWithFreighter(
  invoice: Invoice,
  payer: string,
  options: { signal?: AbortSignal } = {},
): Promise<InvoiceStatusResult> {
  await payInvoiceWithFreighter(invoice, payer);
  return pollInvoiceUntilPaid(invoice.id, { signal: options.signal });
}
