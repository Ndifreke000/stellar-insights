"use client";

import React, {  useEffect, useRef, useState } from "react";
import { useForm, SubmitHandler } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  ExternalLink,
  Loader2,
  RefreshCw,
  AlertCircle,
  CheckCircle,
  Clock,
  Banknote,
} from "lucide-react";
import { FormField, FormSelect } from "@/components/ui/FormField";
import { sep24FlowSchema, type Sep24FlowForm } from "@/lib/schemas";
import {
  useSep24Anchors,
  useSep24Info,
  useSep24Transactions,
  useStartDepositFlow,
  useStartWithdrawFlow,
  useSep24FlowState,
} from "@/hooks/useSep24";
import type { Sep24AnchorInfo, Sep24Transaction } from "@/services/sep24";

type FlowKind = "deposit" | "withdraw";

export function Sep24Flow() {
  // React Query hooks
  const { data: anchorsResponse, isLoading: loadingAnchors, error: anchorsError } = useSep24Anchors();
  const anchors: Sep24AnchorInfo[] = anchorsResponse?.anchors ?? [];

  // Mutation hooks
  const startDepositMutation = useStartDepositFlow();
  const startWithdrawMutation = useStartWithdrawFlow();

  // Zustand state
  const { formData } = useSep24FlowState();

  // Local state
  const [flowKind, setFlowKind] = useState<FlowKind>("deposit");
  const [error, setError] = useState<string | null>(null);
  const [interactiveUrl, setInteractiveUrl] = useState<string | null>(null);
  const [selectedAnchor, setSelectedAnchor] = useState<Sep24AnchorInfo | null>(null);
  const [pollingTimedOut, setPollingTimedOut] = useState(false);
  const pollingTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const POLLING_TIMEOUT_MS = 10 * 60 * 1000;

  const {
    handleSubmit,
    formState: { isValid, isDirty },
    setValue,
    watch,
    trigger,
  } = useForm<Sep24FlowForm>({
    resolver: zodResolver(sep24FlowSchema),
    mode: "onChange",
    defaultValues: formData,
  });

  // Watch form values for real-time updates
  const watchedTransferServer = watch("transferServer");
  const assetCode = watch("assetCode");
  const _amount = watch("amount");
  const _account = watch("account");
  const jwt = watch("jwt");

  // Update selected anchor when transfer server changes
  useEffect(() => {
    if (watchedTransferServer && anchors) {
      const anchor = anchors.find((a) => a.transfer_server === watchedTransferServer);
      setSelectedAnchor(anchor || null);
    } else {
      setSelectedAnchor(null);
    }
  }, [watchedTransferServer, anchors]);

  // Resolved transfer server: prefer the selected anchor's, falling back to
  // whatever was typed manually into the form field.
  const transferServer = selectedAnchor?.transfer_server || watchedTransferServer?.trim();

  const { data: info, isLoading: loadingInfo } = useSep24Info(transferServer);
  const {
    data: transactionsResponse,
    isLoading: loadingTx,
    refetch: loadTransactions,
  } = useSep24Transactions(transferServer, jwt);
  const transactions: Sep24Transaction[] = transactionsResponse?.transactions ?? [];

  const isFormValid = isValid && isDirty;

  React.useEffect(() => {
    if (anchorsError) {
      setError(anchorsError.message);
    }
  }, [anchorsError]);

  React.useEffect(() => {
    if (info && assetCode && info.deposit && info.withdraw) {
      const assets = flowKind === "deposit" ? info.deposit : info.withdraw;
      const codes = Object.keys(assets);
      if (!codes.includes(assetCode)) {
        setValue("assetCode", codes[0]);
      }
    }
  }, [info, flowKind, assetCode, setValue]);

  const assets = info
    ? flowKind === "deposit"
      ? info.deposit
      : info.withdraw
    : null;
  const assetCodes = assets ? Object.keys(assets) : [];

  useEffect(() => {
    return () => {
      if (pollingTimeoutRef.current) {
        clearTimeout(pollingTimeoutRef.current);
      }
    };
  }, []);

  const startFlow: SubmitHandler<Sep24FlowForm> = async (data) => {
    if (!isFormValid) {
      return;
    }
    const base = selectedAnchor?.transfer_server || data.transferServer;
    if (!base) {
      setError("Select an anchor or enter a transfer server URL");
      return;
    }

    setError(null);
    setInteractiveUrl(null);
    setPollingTimedOut(false);

    if (pollingTimeoutRef.current) {
      clearTimeout(pollingTimeoutRef.current);
    }
    pollingTimeoutRef.current = setTimeout(() => {
      setPollingTimedOut(true);
    }, POLLING_TIMEOUT_MS);

    const mutation = flowKind === "deposit" ? startDepositMutation : startWithdrawMutation;

    const params = {
      transferServer: base,
      assetCode: data.assetCode || undefined,
      account: data.account || undefined,
      amount: data.amount || undefined,
      jwt: data.jwt || undefined,
    };

    mutation.mutate(params);
  };

  const statusColor = (status: string) => {
    const s = status?.toLowerCase() || "";
    if (s.includes("complete") || s.includes("success")) return "text-emerald-400";
    if (s.includes("pending") || s.includes("processing")) return "text-amber-400";
    return "text-muted-foreground";
  };

  return (
    <div className="space-y-8">
      {/* Anchor selection */}
      <section className="glass-card rounded-2xl p-6">
        <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
          <Banknote className="w-5 h-5 text-accent" />
          Anchor & transfer server
        </h2>
        <div className="flex flex-wrap gap-4 items-end">
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-muted-foreground mb-1">
              Preset anchors
            </label>
            <select
              className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground focus:ring-2 focus:ring-accent/50"
              value={selectedAnchor?.transfer_server ?? ""}
              onChange={(e: React.ChangeEvent<HTMLSelectElement>) => {
                const a = anchors.find((x: Sep24AnchorInfo) => x.transfer_server === e.target.value);
                setSelectedAnchor(a || null);
                setValue("transferServer", e.target.value);
                trigger("transferServer");
              }}
              disabled={loadingAnchors}
            >
              <option value="">Select an anchor</option>
              {anchors.map((a) => (
                <option key={a.transfer_server} value={a.transfer_server}>
                  {a.name} ({a.transfer_server})
                </option>
              ))}
            </select>
          </div>
          <button
            type="button"
            onClick={() => window.location.reload()}
            disabled={loadingAnchors}
            className="rounded-xl border border-border px-4 py-2.5 text-sm font-medium text-foreground hover:bg-white/5 flex items-center gap-2"
          >
            {loadingAnchors ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            Refresh
          </button>
        </div>
        <div className="mt-4">
          <FormField
            name="transferServer"
            label="Or enter transfer server URL"
            type="url"
            placeholder="https://api.anchor.example/sep24"
            description="Custom SEP-24 transfer server endpoint"
          />
        </div>
      </section>

      {/* Deposit / Withdraw toggle and form */}
      <section className="glass-card rounded-2xl p-6">
        <h2 className="text-lg font-semibold text-foreground mb-4">
          Start flow
        </h2>
        <div className="flex gap-2 mb-4">
          <button
            type="button"
            onClick={() => setFlowKind("deposit")}
            className={`flex items-center gap-2 rounded-xl px-4 py-2.5 font-medium transition-all ${flowKind === "deposit"
              ? "bg-accent/20 text-accent border border-accent/30"
              : "border border-border text-muted-foreground hover:bg-white/5"
              }`}
          >
            <ArrowDownToLine className="w-4 h-4" />
            Deposit
          </button>
          <button
            type="button"
            onClick={() => setFlowKind("withdraw")}
            className={`flex items-center gap-2 rounded-xl px-4 py-2.5 font-medium transition-all ${flowKind === "withdraw"
              ? "bg-accent/20 text-accent border border-accent/30"
              : "border border-border text-muted-foreground hover:bg-white/5"
              }`}
          >
            <ArrowUpFromLine className="w-4 h-4" />
            Withdraw
          </button>
        </div>

        {info && (
          <>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
              <FormSelect
                name="assetCode"
                label="Asset"
                options={assetCodes.map((c) => ({ value: c, label: c }))}
                required
              />
              <FormField
                name="amount"
                label="Amount (optional)"
                type="text"
                placeholder="0.00"
                description="Leave empty for anchor to determine"
              />
            </div>
            <div className="mb-4">
              <FormField
                name="account"
                label="Stellar account (optional)"
                type="text"
                placeholder="G..."
                description="Your Stellar public key"
              />
            </div>
            <div className="mb-4">
              <FormField
                name="jwt"
                label="JWT (optional, from SEP-10)"
                type="password"
                placeholder="For authenticated flows"
                description="Authentication token from SEP-10 challenge"
              />
            </div>
            {error && (
              <div className="mb-4 flex items-center gap-2 rounded-xl bg-red-500/10 border border-red-500/20 px-4 py-3 text-red-400 text-sm">
                <AlertCircle className="w-4 h-4 shrink-0" />
                {error}
              </div>
            )}
            {interactiveUrl && (
              <div className="mb-4 flex items-center gap-2 rounded-xl bg-emerald-500/10 border border-emerald-500/20 px-4 py-3 text-emerald-400 text-sm">
                <CheckCircle className="w-4 h-4 shrink-0" />
                Interactive window opened. Complete the flow there.
                <a
                  href={interactiveUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="ml-2 inline-flex items-center gap-1 text-accent hover:underline"
                >
                  Open again <ExternalLink className="w-3 h-3" />
                </a>
              </div>
            )}
            {pollingTimedOut && (
              <div className="mb-4 flex items-center gap-2 rounded-xl bg-amber-500/10 border border-amber-500/20 px-4 py-3 text-amber-400 text-sm">
                <AlertCircle className="w-4 h-4 shrink-0" />
                Anchor did not respond — check your email or contact support.
              </div>
            )}
            <button
              type="button"
              onClick={handleSubmit(startFlow)}
              disabled={startDepositMutation.isPending || startWithdrawMutation.isPending || !isFormValid}
              className="rounded-xl bg-accent text-accent-foreground px-6 py-2.5 font-medium hover:opacity-90 flex items-center gap-2 disabled:opacity-50"
            >
              {(startDepositMutation.isPending || startWithdrawMutation.isPending) ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : flowKind === "deposit" ? (
                <ArrowDownToLine className="w-4 h-4" />
              ) : (
                <ArrowUpFromLine className="w-4 h-4" />
              )}
              {flowKind === "deposit" ? "Start deposit" : "Start withdrawal"}
            </button>
          </>
        )}

        {transferServer && loadingInfo && (
          <div className="flex items-center gap-2 text-muted-foreground py-4">
            <Loader2 className="w-4 h-4 animate-spin" />
            Loading anchor capabilities…
          </div>
        )}
        {transferServer && !loadingInfo && !info && !error && (
          <p className="text-muted-foreground py-4">
            Could not load anchor info. Check the URL and CORS/allowed origins.
          </p>
        )}
      </section>

      {/* Transaction history */}
      <section className="glass-card rounded-2xl p-6">
        <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
          <Clock className="w-5 h-5 text-accent" />
          Transaction history
        </h2>
        {transferServer && (
          <button
            type="button"
            onClick={() => loadTransactions()}
            disabled={loadingTx}
            className="mb-4 rounded-xl border border-border px-4 py-2 text-sm font-medium text-foreground hover:bg-white/5 flex items-center gap-2"
          >
            {loadingTx ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            Load history
          </button>
        )}
        {transactions.length === 0 && !loadingTx && (
          <p className="text-muted-foreground text-sm">
            {transferServer
              ? "Click 'Load history' to fetch transactions (JWT may be required)."
              : "Select an anchor above to load transaction history."}
          </p>
        )}
        {transactions.length > 0 && (
          <div className="overflow-x-auto rounded-xl border border-border">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-white/5">
                  <th className="text-left py-3 px-4 font-medium text-muted-foreground">
                    Kind
                  </th>
                  <th className="text-left py-3 px-4 font-medium text-muted-foreground">
                    Asset
                  </th>
                  <th className="text-left py-3 px-4 font-medium text-muted-foreground">
                    Amount
                  </th>
                  <th className="text-left py-3 px-4 font-medium text-muted-foreground">
                    Status
                  </th>
                  <th className="text-left py-3 px-4 font-medium text-muted-foreground">
                    Date
                  </th>
                </tr>
              </thead>
              <tbody>
                {transactions.map((tx: Sep24Transaction) => (
                  <tr
                    key={tx.id}
                    className="border-b border-border/50 hover:bg-white/5"
                  >
                    <td className="py-3 px-4 capitalize text-foreground">
                      {tx.kind}
                    </td>
                    <td className="py-3 px-4 text-foreground">
                      {tx.asset_code ?? "—"}
                    </td>
                    <td className="py-3 px-4 text-foreground">
                      {tx.amount_in ?? tx.amount_out ?? "—"}
                    </td>
                    <td className={`py-3 px-4 ${statusColor(tx.status)}`}>
                      {tx.status}
                    </td>
                    <td className="py-3 px-4 text-muted-foreground">
                      {tx.started_at
                        ? new Date(tx.started_at).toLocaleString()
                        : "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}
