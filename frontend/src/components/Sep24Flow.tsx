"use client";

import React, { useCallback, useState } from "react";
import {
  getSep24Info,
  startDepositInteractive,
  startWithdrawInteractive,
  getSep24Transactions,
  getSep24Anchors,
  type Sep24AnchorInfo,
  type Sep24InfoResponse,
  type Sep24Transaction,
  Sep24Error,
} from "@/services/sep24";
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

type FlowKind = "deposit" | "withdraw";

export function Sep24Flow() {
  const [anchors, setAnchors] = useState<Sep24AnchorInfo[]>([]);
  const [selectedAnchor, setSelectedAnchor] = useState<Sep24AnchorInfo | null>(
    null
  );
  const [info, setInfo] = useState<Sep24InfoResponse | null>(null);
  const [transactions, setTransactions] = useState<Sep24Transaction[]>([]);
  const [loadingAnchors, setLoadingAnchors] = useState(false);
  const [loadingInfo, setLoadingInfo] = useState(false);
  const [loadingTx, setLoadingTx] = useState(false);
  const [flowKind, setFlowKind] = useState<FlowKind>("deposit");
  const [assetCode, setAssetCode] = useState("");
  const [amount, setAmount] = useState("");
  const [account, setAccount] = useState("");
  const [jwt, setJwt] = useState("");
  const [customTransferServer, setCustomTransferServer] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [interactiveUrl, setInteractiveUrl] = useState<string | null>(null);
  const [startingFlow, setStartingFlow] = useState(false);

  const loadAnchors = useCallback(async () => {
    setLoadingAnchors(true);
    setError(null);
    try {
      const res = await getSep24Anchors();
      setAnchors(res.anchors || []);
      if (res.anchors?.length && !selectedAnchor) {
        setSelectedAnchor(res.anchors[0]);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load anchors");
    } finally {
      setLoadingAnchors(false);
    }
  }, [selectedAnchor]);

  const loadInfo = useCallback(async () => {
    const base = selectedAnchor?.transfer_server || customTransferServer?.trim();
    if (!base) {
      setInfo(null);
      return;
    }
    setLoadingInfo(true);
    setError(null);
    try {
      const data = await getSep24Info(base);
      setInfo(data);
      const assets = flowKind === "deposit" ? data.deposit : data.withdraw;
      const codes = assets ? Object.keys(assets) : [];
      setAssetCode((prev) => (codes.length && (!prev || !codes.includes(prev)) ? codes[0] : prev));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load anchor info");
      setInfo(null);
    } finally {
      setLoadingInfo(false);
    }
  }, [selectedAnchor, customTransferServer, flowKind]);

  const loadTransactions = useCallback(async () => {
    const base = selectedAnchor?.transfer_server || customTransferServer?.trim();
    if (!base) return;
    setLoadingTx(true);
    setError(null);
    try {
      const res = await getSep24Transactions({
        transfer_server: base,
        jwt: jwt || undefined,
        kind: flowKind,
        limit: 20,
      });
      setTransactions(res.transactions || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load transactions");
      setTransactions([]);
    } finally {
      setLoadingTx(false);
    }
  }, [selectedAnchor, customTransferServer, jwt, flowKind]);

  React.useEffect(() => {
    loadAnchors();
  }, []);

  React.useEffect(() => {
    if (selectedAnchor?.transfer_server || customTransferServer?.trim()) {
      loadInfo();
    } else {
      setInfo(null);
    }
  }, [selectedAnchor, customTransferServer, flowKind]);

  const transferServer = selectedAnchor?.transfer_server || customTransferServer?.trim();
  const assets = info
    ? flowKind === "deposit"
      ? info.deposit
      : info.withdraw
    : null;
  const assetCodes = assets ? Object.keys(assets) : [];

  const startFlow = async () => {
    if (!transferServer) {
      setError("Select an anchor or enter a transfer server URL");
      return;
    }
    setStartingFlow(true);
    setError(null);
    setInteractiveUrl(null);
    try {
      const params = {
        transfer_server: transferServer,
        asset_code: assetCode || undefined,
        account: account || undefined,
        amount: amount || undefined,
        jwt: jwt || undefined,
      };
      const res =
        flowKind === "deposit"
          ? await startDepositInteractive(params)
          : await startWithdrawInteractive(params);
      const url =
        (res as { url?: string }).url ||
        (res as { transaction?: { id: string } })?.transaction?.id;
      if (typeof url === "string" && url.startsWith("http")) {
        setInteractiveUrl(url);
        window.open(url, "sep24-interactive", "width=500,height=700");
      } else {
        setError("Anchor did not return an interactive URL. Try again or complete KYC on the anchor site.");
      }
    } catch (e) {
      const msg =
        e instanceof Sep24Error
          ? e.message
          : e instanceof Error
            ? e.message
            : "Failed to start flow";
      setError(msg);
    } finally {
      setStartingFlow(false);
    }
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
            placeholder="https://api.anchor.example/sep24"
            className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground focus:ring-2 focus:ring-accent/50"
            value={customTransferServer}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
              setCustomTransferServer(e.target.value);
              setSelectedAnchor(null);
            }}
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

        {info && (
          <>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
              <div>
                <label className="block text-sm font-medium text-muted-foreground mb-1">
                  Asset
                </label>
                <select
                  className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground"
                  value={assetCode}
                  onChange={(e: React.ChangeEvent<HTMLSelectElement>) => setAssetCode(e.target.value)}
                >
                  {assetCodes.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-muted-foreground mb-1">
                  Amount (optional)
                </label>
                <input
                  type="text"
                  placeholder="0.00"
                  className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground"
                  value={amount}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) => setAmount(e.target.value)}
                />
              </div>
            </div>
            <div className="mb-4">
              <label className="block text-sm font-medium text-muted-foreground mb-1">
                Stellar account (optional)
              </label>
              <input
                type="text"
                placeholder="G..."
                className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground font-mono text-sm"
                value={account}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setAccount(e.target.value)}
              />
            </div>
            <div className="mb-4">
              <label className="block text-sm font-medium text-muted-foreground mb-1">
                JWT (optional, from SEP-10)
              </label>
              <input
                type="password"
                placeholder="For authenticated flows"
                className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground font-mono text-sm"
                value={jwt}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setJwt(e.target.value)}
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
            <button
              type="button"
              onClick={startFlow}
              disabled={startingFlow}
              className="rounded-xl bg-accent text-accent-foreground px-6 py-2.5 font-medium hover:opacity-90 flex items-center gap-2 disabled:opacity-50"
            >
              {startingFlow ? (
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
            onClick={loadTransactions}
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
              ? "Click “Load history” to fetch transactions (JWT may be required)."
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
