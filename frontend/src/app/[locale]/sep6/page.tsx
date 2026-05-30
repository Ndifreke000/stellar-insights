"use client";

import React, { useCallback, useEffect, useState } from "react";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  Banknote,
  Loader2,
  RefreshCw,
  AlertCircle,
  Clock,
  CheckCircle,
  XCircle,
} from "lucide-react";
import {
  getSep6Anchors,
  getSep6Info,
  getSep6Transactions,
  getSep6Transaction,
  type Sep6AnchorInfo,
  type Sep6InfoResponse,
  type Sep6Transaction,
  Sep6Error,
} from "@/services/sep6";
import { Sep6DepositForm } from "@/components/Sep6DepositForm";
import { Sep6WithdrawForm } from "@/components/Sep6WithdrawForm";

type FlowKind = "deposit" | "withdraw";

function statusColor(status: string) {
  const s = status?.toLowerCase() || "";
  if (s.includes("complete") || s.includes("success"))
    return "text-emerald-400";
  if (s.includes("pending") || s.includes("processing"))
    return "text-amber-400";
  if (s.includes("error") || s.includes("failed")) return "text-red-400";
  return "text-muted-foreground";
}

function statusIcon(status: string) {
  const s = status?.toLowerCase() || "";
  if (s.includes("complete") || s.includes("success"))
    return <CheckCircle className="w-4 h-4" />;
  if (s.includes("error") || s.includes("failed"))
    return <XCircle className="w-4 h-4" />;
  return <Clock className="w-4 h-4" />;
}

export default function Sep6Page() {
  const [anchors, setAnchors] = useState<Sep6AnchorInfo[]>([]);
  const [selectedAnchor, setSelectedAnchor] = useState<Sep6AnchorInfo | null>(
    null
  );
  const [customTransferServer, setCustomTransferServer] = useState("");
  const [info, setInfo] = useState<Sep6InfoResponse | null>(null);
  const [transactions, setTransactions] = useState<Sep6Transaction[]>([]);
  const [txDetail, setTxDetail] = useState<Sep6Transaction | null>(null);
  const [txIdInput, setTxIdInput] = useState("");
  const [loadingAnchors, setLoadingAnchors] = useState(false);
  const [loadingInfo, setLoadingInfo] = useState(false);
  const [loadingTx, setLoadingTx] = useState(false);
  const [loadingTxDetail, setLoadingTxDetail] = useState(false);
  const [flowKind, setFlowKind] = useState<FlowKind>("deposit");
  const [jwt, setJwt] = useState("");
  const [error, setError] = useState<string | null>(null);

  const transferServer =
    selectedAnchor?.transfer_server || customTransferServer.trim();

  const loadAnchors = useCallback(async () => {
    setLoadingAnchors(true);
    setError(null);
    try {
      const res = await getSep6Anchors();
      setAnchors(res.anchors || []);
      if (res.anchors?.length && !selectedAnchor) {
        setSelectedAnchor(res.anchors[0]);
      }
    } catch (e) {
      setError(
        e instanceof Error ? e.message : "Failed to load anchors. Is the backend proxy running?"
      );
    } finally {
      setLoadingAnchors(false);
    }
  }, [selectedAnchor]);

  const loadInfo = useCallback(async () => {
    if (!transferServer) {
      setInfo(null);
      return;
    }
    setLoadingInfo(true);
    setError(null);
    try {
      const data = await getSep6Info(transferServer);
      setInfo(data);
    } catch (e) {
      setError(
        e instanceof Error ? e.message : "Failed to load anchor info"
      );
      setInfo(null);
    } finally {
      setLoadingInfo(false);
    }
  }, [transferServer]);

  const loadTransactions = useCallback(async () => {
    if (!transferServer) return;
    setLoadingTx(true);
    setError(null);
    try {
      const res = await getSep6Transactions({
        transfer_server: transferServer,
        jwt: jwt || undefined,
        kind: flowKind,
        limit: 20,
      });
      setTransactions(res.transactions || []);
    } catch (e) {
      setError(
        e instanceof Sep6Error ? e.message : "Failed to load transactions"
      );
      setTransactions([]);
    } finally {
      setLoadingTx(false);
    }
  }, [transferServer, jwt, flowKind]);

  const fetchTxById = useCallback(async () => {
    const id = txIdInput.trim();
    if (!transferServer || !id) return;
    setLoadingTxDetail(true);
    setError(null);
    setTxDetail(null);
    try {
      const res = await getSep6Transaction(transferServer, id, jwt || undefined);
      setTxDetail(res.transaction);
    } catch (e) {
      setError(
        e instanceof Sep6Error ? e.message : "Transaction not found"
      );
    } finally {
      setLoadingTxDetail(false);
    }
  }, [transferServer, txIdInput, jwt]);

  useEffect(() => {
    loadAnchors();
  }, []);

  useEffect(() => {
    if (transferServer) loadInfo();
    else setInfo(null);
  }, [transferServer]);

  const depositAssets = info?.deposit ? Object.keys(info.deposit) : [];
  const withdrawAssets = info?.withdraw ? Object.keys(info.withdraw) : [];
  const withdrawTypes = info?.withdraw
    ? Array.from(
        new Set(
          Object.values(info.withdraw)
            .map((m) => m.type)
            .filter(Boolean)
        )
      ) as string[]
    : [];
  if (withdrawTypes.length === 0) withdrawTypes.push("bank_account");

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            SEP-6 // Programmatic
          </div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <span className="flex items-center gap-2">
              <ArrowDownToLine className="w-8 h-8 text-emerald-500/80" />
              <ArrowUpFromLine className="w-8 h-8 text-amber-500/80" />
            </span>
            Deposit & Withdraw (SEP-6)
          </h2>
        </div>
        <p className="text-muted-foreground text-sm max-w-md">
          Programmatic deposit and withdrawal via anchor transfer servers. Request instructions and track transaction status.
        </p>
      </div>

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
              onChange={(e) => {
                const a = anchors.find(
                  (x) => x.transfer_server === e.target.value
                );
                setSelectedAnchor(a || null);
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
            onClick={loadAnchors}
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
          <label className="block text-sm font-medium text-muted-foreground mb-1">
            Or enter transfer server URL
          </label>
          <input
            type="url"
            placeholder="https://api.anchor.example/sep6"
            className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground focus:ring-2 focus:ring-accent/50"
            value={customTransferServer}
            onChange={(e) => {
              setCustomTransferServer(e.target.value);
              setSelectedAnchor(null);
            }}
          />
        </div>
      </section>

      {/* Deposit / Withdraw forms */}
      <section className="glass-card rounded-2xl p-6">
        <h2 className="text-lg font-semibold text-foreground mb-4">
          Request
        </h2>
        <div className="flex gap-2 mb-6">
          <button
            type="button"
            onClick={() => setFlowKind("deposit")}
            className={`flex items-center gap-2 rounded-xl px-4 py-2.5 font-medium transition-all ${
              flowKind === "deposit"
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
            className={`flex items-center gap-2 rounded-xl px-4 py-2.5 font-medium transition-all ${
              flowKind === "withdraw"
                ? "bg-accent/20 text-accent border border-accent/30"
                : "border border-border text-muted-foreground hover:bg-white/5"
            }`}
          >
            <ArrowUpFromLine className="w-4 h-4" />
            Withdraw
          </button>
        </div>

        {transferServer && loadingInfo && (
          <div className="flex items-center gap-2 text-muted-foreground py-4">
            <Loader2 className="w-4 h-4 animate-spin" />
            Loading anchor capabilities…
          </div>
        )}

        {transferServer && !loadingInfo && info && (
          <>
            {flowKind === "deposit" ? (
              <Sep6DepositForm
                transferServer={transferServer}
                assetCodes={depositAssets}
                defaultAssetCode={depositAssets[0]}
                jwt={jwt}
              />
            ) : (
              <Sep6WithdrawForm
                transferServer={transferServer}
                assetCodes={withdrawAssets}
                withdrawTypes={withdrawTypes}
                defaultAssetCode={withdrawAssets[0]}
                defaultType={withdrawTypes[0]}
                jwt={jwt}
              />
            )}
          </>
        )}

        {transferServer && !loadingInfo && !info && !error && (
          <p className="text-muted-foreground py-4">
            Could not load anchor info. Check the URL and ensure the backend SEP-6 proxy is configured.
          </p>
        )}

        <div className="mt-6 pt-6 border-t border-border">
          <label className="block text-sm font-medium text-muted-foreground mb-1">
            JWT (optional, for authenticated flows)
          </label>
          <input
            type="password"
            placeholder="SEP-10 JWT"
            className="w-full max-w-md rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground font-mono text-sm"
            value={jwt}
            onChange={(e) => setJwt(e.target.value)}
          />
        </div>
      </section>

      {/* Transaction status */}
      <section className="glass-card rounded-2xl p-6">
        <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
          <Clock className="w-5 h-5 text-accent" />
          Transaction status
        </h2>
        <div className="flex flex-wrap gap-4 items-end mb-4">
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-muted-foreground mb-1">
              Look up by ID
            </label>
            <input
              type="text"
              placeholder="Transaction id"
              className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground font-mono text-sm"
              value={txIdInput}
              onChange={(e) => setTxIdInput(e.target.value)}
            />
          </div>
          <button
            type="button"
            onClick={fetchTxById}
            disabled={!transferServer || !txIdInput.trim() || loadingTxDetail}
            className="rounded-xl border border-border px-4 py-2.5 text-sm font-medium text-foreground hover:bg-white/5 flex items-center gap-2 disabled:opacity-50"
          >
            {loadingTxDetail ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              "Fetch"
            )}
          </button>
        </div>
        {txDetail && (
          <div className="rounded-xl bg-white/5 border border-border p-4 mb-4 space-y-2 text-sm">
            <div className="flex items-center gap-2">
              {statusIcon(txDetail.status)}
              <span className={statusColor(txDetail.status)}>
                {txDetail.status}
              </span>
              <span className="text-muted-foreground font-mono text-xs">
                {txDetail.id}
              </span>
            </div>
            {txDetail.asset_code && (
              <p className="text-muted-foreground">
                Asset: <span className="text-foreground">{txDetail.asset_code}</span>
              </p>
            )}
            {txDetail.amount_in && (
              <p className="text-muted-foreground">
                Amount in: {txDetail.amount_in}
              </p>
            )}
            {txDetail.amount_out && (
              <p className="text-muted-foreground">
                Amount out: {txDetail.amount_out}
              </p>
            )}
            {txDetail.completed_at && (
              <p className="text-muted-foreground text-xs">
                Completed: {txDetail.completed_at}
              </p>
            )}
          </div>
        )}

        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-medium text-muted-foreground">
            Recent transactions
          </span>
          <button
            type="button"
            onClick={loadTransactions}
            disabled={!transferServer || loadingTx}
            className="text-sm text-accent hover:underline flex items-center gap-1 disabled:opacity-50"
          >
            {loadingTx ? (
              <Loader2 className="w-3 h-3 animate-spin" />
            ) : (
              <RefreshCw className="w-3 h-3" />
            )}
            Refresh
          </button>
        </div>
        {transactions.length === 0 && !loadingTx && (
          <p className="text-muted-foreground text-sm py-2">
            No transactions loaded. Use JWT for authenticated anchors.
          </p>
        )}
        <ul className="space-y-2">
          {transactions.map((tx) => (
            <li
              key={tx.id}
              className="flex items-center justify-between gap-4 rounded-xl bg-white/5 border border-border px-4 py-3 text-sm"
            >
              <div className="flex items-center gap-2 min-w-0">
                {statusIcon(tx.status)}
                <span className={statusColor(tx.status)}>{tx.status}</span>
                <span className="text-muted-foreground truncate font-mono text-xs">
                  {tx.id}
                </span>
              </div>
              <div className="shrink-0 text-muted-foreground">
                {tx.asset_code && `${tx.asset_code} · `}
                {tx.amount_in ?? tx.amount_out ?? "—"}
              </div>
            </li>
          ))}
        </ul>
      </section>

      {error && (
        <div className="flex items-center gap-2 rounded-xl bg-red-500/10 border border-red-500/20 px-4 py-3 text-red-400 text-sm">
          <AlertCircle className="w-4 h-4 shrink-0" />
          {error}
        </div>
      )}
    </div>
  );
}
