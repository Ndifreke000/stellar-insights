"use client";

import { useEffect, useState } from "react";
import { CheckCircle, Copy, Loader2, Zap } from "lucide-react";
import {
  confirmInvoice,
  createInvoice,
  getBillingPlan,
  getSubscription,
  type ApiKeyInfo,
  type BillingPlan,
  type Invoice,
  type Subscription,
} from "@/lib/api-keys";
import { payInvoiceWithFreighter } from "@/lib/pay-invoice";
import { logger } from "@/lib/logger";

interface UpgradePanelProps {
  authToken: string;
  address: string;
  keys: ApiKeyInfo[];
}

function CopyField({ label, value }: { label: string; value: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div>
      <div className="text-[10px] font-mono uppercase tracking-widest text-muted-foreground mb-1">
        {label}
      </div>
      <div className="flex items-center gap-2">
        <code className="flex-1 truncate rounded-lg bg-muted/40 px-3 py-2 text-xs font-mono text-foreground">
          {value}
        </code>
        <button
          type="button"
          aria-label={`Copy ${label}`}
          onClick={() => {
            void navigator.clipboard?.writeText(value);
            setCopied(true);
            setTimeout(() => setCopied(false), 1500);
          }}
          className="rounded-lg border border-border p-2 hover:bg-muted/40"
        >
          {copied ? <CheckCircle className="h-4 w-4 text-green-500" aria-hidden="true" /> : <Copy className="h-4 w-4" aria-hidden="true" />}
        </button>
      </div>
    </div>
  );
}

/**
 * Upgrade an API key to the paid plan by paying USDC on Stellar. The page
 * asks the backend for an invoice, the user pays it (with Freighter, or from
 * any wallet using the shown details), and the backend verifies the payment
 * on the ledger before raising the key's limit.
 */
export default function UpgradePanel({ authToken, address, keys }: UpgradePanelProps) {
  const activeKeys = keys.filter((k) => k.status === "active");
  const [plan, setPlan] = useState<BillingPlan | null | undefined>(undefined);
  const [keyId, setKeyId] = useState("");
  const [invoice, setInvoice] = useState<Invoice | null>(null);
  const [subscription, setSubscription] = useState<Subscription | null>(null);
  const [txHash, setTxHash] = useState("");
  const [busy, setBusy] = useState<"invoice" | "pay" | "confirm" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const selectedKey = keyId || activeKeys[0]?.id || "";

  useEffect(() => {
    getBillingPlan()
      .then(setPlan)
      .catch((err) => {
        logger.error("Failed to load billing plan:", err);
        setPlan(null);
      });
  }, []);

  useEffect(() => {
    if (!selectedKey) return;
    getSubscription(authToken, selectedKey)
      .then(setSubscription)
      .catch(() => setSubscription(null));
  }, [authToken, selectedKey]);

  if (plan === undefined) return null;
  if (plan === null) {
    return (
      <div className="glass rounded-xl border border-border p-6 text-sm text-muted-foreground">
        Paid plans are not enabled on this server. Every key gets the free limit.
      </div>
    );
  }

  const run = async (step: "invoice" | "pay" | "confirm", action: () => Promise<void>) => {
    setBusy(step);
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(null);
    }
  };

  const confirm = async (hash: string) => {
    if (!invoice) return;
    const result = await confirmInvoice(authToken, invoice.id, hash);
    setInvoice(result.invoice);
    setSubscription(result.subscription);
  };

  const paidUntil = subscription && new Date(subscription.paid_until);
  const active = paidUntil !== null && paidUntil !== undefined && paidUntil > new Date();

  return (
    <section className="glass rounded-xl border border-border p-6 space-y-6" aria-labelledby="upgrade-heading">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <h3 id="upgrade-heading" className="text-lg font-bold flex items-center gap-2">
            <Zap className="h-5 w-5 text-accent" aria-hidden="true" />
            Pro plan: {plan.limit_per_minute.toLocaleString()} requests a minute
          </h3>
          <p className="text-sm text-muted-foreground">
            {plan.price} {plan.asset_code} per {plan.period_days} days, paid on Stellar. Free keys get{" "}
            {plan.free_limit_per_minute} a minute; without a key the limit is {plan.anonymous_limit_per_minute}.
          </p>
        </div>
        {activeKeys.length > 0 && (
          <label className="text-sm">
            <span className="sr-only">API key to upgrade</span>
            <select
              aria-label="API key to upgrade"
              value={selectedKey}
              onChange={(e) => {
                setKeyId(e.target.value);
                setInvoice(null);
                setTxHash("");
              }}
              className="rounded-lg border border-border bg-background px-3 py-2"
            >
              {activeKeys.map((k) => (
                <option key={k.id} value={k.id}>
                  {k.name} ({k.key_prefix})
                </option>
              ))}
            </select>
          </label>
        )}
      </div>

      {activeKeys.length === 0 && (
        <p className="text-sm text-muted-foreground">Create a key above to upgrade it.</p>
      )}

      {active && paidUntil && (
        <div className="rounded-lg border border-green-500/30 bg-green-500/10 p-3 text-sm text-green-600 dark:text-green-400">
          This key is on the Pro plan until {paidUntil.toLocaleDateString()}. Paying again adds {plan.period_days} days.
        </div>
      )}

      {error && (
        <div role="alert" className="rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {selectedKey && !invoice && (
        <button
          type="button"
          disabled={busy !== null}
          onClick={() => run("invoice", async () => setInvoice(await createInvoice(authToken, selectedKey)))}
          className="inline-flex items-center gap-2 rounded-xl bg-accent px-4 py-2 font-medium text-white disabled:opacity-60"
        >
          {busy === "invoice" && <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />}
          {active ? "Renew" : "Upgrade"} for {plan.price} {plan.asset_code}
        </button>
      )}

      {invoice && invoice.status === "paid" && (
        <div className="rounded-lg border border-green-500/30 bg-green-500/10 p-3 text-sm">
          Payment confirmed on the ledger
          {invoice.transaction_hash && (
            <>
              {" "}
              (<code className="font-mono">{invoice.transaction_hash.slice(0, 12)}…</code>)
            </>
          )}
          . Your new limit applies within a minute.
        </div>
      )}

      {invoice && invoice.status === "pending" && (
        <div className="space-y-4">
          <p className="text-sm text-muted-foreground">
            Send exactly this payment, with the memo, before {new Date(invoice.expires_at).toLocaleString()}.
          </p>
          <div className="grid gap-3 md:grid-cols-2">
            <CopyField label="Amount" value={`${invoice.amount_usdc} ${invoice.asset_code}`} />
            <CopyField label="Memo (text)" value={invoice.memo} />
            <CopyField label="Destination" value={invoice.destination} />
            <CopyField label="Asset issuer" value={invoice.asset_issuer} />
          </div>

          <div className="flex flex-col gap-3 md:flex-row md:items-end">
            <button
              type="button"
              disabled={busy !== null}
              onClick={() =>
                run("pay", async () => {
                  const hash = await payInvoiceWithFreighter(invoice, address);
                  setTxHash(hash);
                  await confirm(hash);
                })
              }
              className="inline-flex items-center gap-2 rounded-xl bg-accent px-4 py-2 font-medium text-white disabled:opacity-60"
            >
              {busy === "pay" && <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />}
              Pay with Freighter
            </button>
            <span className="text-xs text-muted-foreground md:pb-2">or pay from any wallet, then:</span>
            <label className="flex-1">
              <span className="sr-only">Transaction hash</span>
              <input
                value={txHash}
                onChange={(e) => setTxHash(e.target.value)}
                placeholder="Transaction hash"
                aria-label="Transaction hash"
                className="w-full rounded-lg border border-border bg-background px-3 py-2 font-mono text-xs"
              />
            </label>
            <button
              type="button"
              disabled={busy !== null || txHash.trim().length === 0}
              onClick={() => run("confirm", () => confirm(txHash))}
              className="inline-flex items-center gap-2 rounded-xl border border-border px-4 py-2 font-medium disabled:opacity-60"
            >
              {busy === "confirm" && <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />}
              Confirm payment
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
