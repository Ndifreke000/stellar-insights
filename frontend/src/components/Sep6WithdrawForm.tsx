"use client";

import React, { useState } from "react";
import {
  ArrowUpFromLine,
  Loader2,
  AlertCircle,
  CheckCircle,
  ExternalLink,
} from "lucide-react";
import { getSep6Withdraw, Sep6WithdrawResponse, Sep6Error } from "@/services/sep6";

function validateStellarAccount(addr: string): string | null {
  const s = (addr || "").trim();
  if (!s) return "Address is required.";
  if (s.startsWith("G") && s.length === 56 && /^[A-Z0-9]+$/.test(s))
    return null;
  if (s.startsWith("M") && s.length === 69 && /^[A-Z0-9]+$/.test(s))
    return null;
  if (s.startsWith("G"))
    return "Invalid G-address: must be 56 characters.";
  if (s.startsWith("M"))
    return "Invalid M-address: must be 69 characters.";
  return "Invalid address: must start with G (account) or M (muxed).";
}

export interface Sep6WithdrawFormProps {
  transferServer: string;
  assetCodes: string[];
  withdrawTypes: string[];
  defaultAssetCode?: string;
  defaultType?: string;
  jwt?: string;
  onSuccess?: (result: Sep6WithdrawResponse) => void;
}

export function Sep6WithdrawForm({
  transferServer,
  assetCodes,
  withdrawTypes,
  defaultAssetCode = "",
  defaultType = "",
  jwt = "",
  onSuccess,
}: Sep6WithdrawFormProps) {
  const [assetCode, setAssetCode] = useState(
    defaultAssetCode || (assetCodes[0] ?? "")
  );
  const [withdrawType, setWithdrawType] = useState(
    defaultType || (withdrawTypes[0] ?? "")
  );
  const [account, setAccount] = useState("");
  const [dest, setDest] = useState("");
  const [destExtra, setDestExtra] = useState("");
  const [amount, setAmount] = useState("");
  const [memo, setMemo] = useState("");
  const [memoType, setMemoType] = useState("text");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<Sep6WithdrawResponse | null>(null);

  const accountError = validateStellarAccount(account);
  const canSubmit =
    transferServer &&
    assetCode &&
    withdrawType &&
    account &&
    !accountError &&
    !loading;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const res = await getSep6Withdraw({
        transfer_server: transferServer,
        asset_code: assetCode,
        type: withdrawType,
        account: account.trim(),
        dest: dest.trim() || undefined,
        dest_extra: destExtra.trim() || undefined,
        amount: amount.trim() || undefined,
        memo: memo.trim() || undefined,
        memo_type: memoType || undefined,
        jwt: jwt || undefined,
      });
      setResult(res);
      onSuccess?.(res);
    } catch (e) {
      const msg =
        e instanceof Sep6Error
          ? e.message
          : e instanceof Error
            ? e.message
            : "Withdrawal request failed";
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium text-muted-foreground mb-1">
            Asset <span className="text-red-400">*</span>
          </label>
          <select
            className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground focus:ring-2 focus:ring-accent/50"
            value={assetCode}
            onChange={(e) => setAssetCode(e.target.value)}
            required
          >
            <option value="">Select asset</option>
            {assetCodes.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="block text-sm font-medium text-muted-foreground mb-1">
            Withdrawal type <span className="text-red-400">*</span>
          </label>
          <select
            className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground focus:ring-2 focus:ring-accent/50"
            value={withdrawType}
            onChange={(e) => setWithdrawType(e.target.value)}
            required
          >
            <option value="">Select type</option>
            {withdrawTypes.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div>
        <label className="block text-sm font-medium text-muted-foreground mb-1">
          Stellar account <span className="text-red-400">*</span>
        </label>
        <input
          type="text"
          placeholder="G... or M..."
          className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground font-mono text-sm"
          value={account}
          onChange={(e) => {
            setAccount(e.target.value);
            setError(null);
          }}
          required
        />
        {account && accountError && (
          <p className="mt-1 text-xs text-red-400">{accountError}</p>
        )}
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium text-muted-foreground mb-1">
            Destination (optional)
          </label>
          <input
            type="text"
            placeholder="Bank account, wallet, etc."
            className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground"
            value={dest}
            onChange={(e) => setDest(e.target.value)}
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-muted-foreground mb-1">
            Destination extra (optional)
          </label>
          <input
            type="text"
            placeholder="e.g. routing number"
            className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground"
            value={destExtra}
            onChange={(e) => setDestExtra(e.target.value)}
          />
        </div>
      </div>

      <div>
        <label className="block text-sm font-medium text-muted-foreground mb-1">
          Amount (optional)
        </label>
        <input
          type="text"
          inputMode="decimal"
          placeholder="0.00"
          className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
        />
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div>
          <label className="block text-sm font-medium text-muted-foreground mb-1">
            Memo (optional)
          </label>
          <input
            type="text"
            placeholder="Memo"
            className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground placeholder:text-muted-foreground"
            value={memo}
            onChange={(e) => setMemo(e.target.value)}
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-muted-foreground mb-1">
            Memo type
          </label>
          <select
            className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground"
            value={memoType}
            onChange={(e) => setMemoType(e.target.value)}
          >
            <option value="text">text</option>
            <option value="id">id</option>
            <option value="hash">hash</option>
          </select>
        </div>
      </div>

      {error && (
        <div className="flex items-center gap-2 rounded-xl bg-red-500/10 border border-red-500/20 px-4 py-3 text-red-400 text-sm">
          <AlertCircle className="w-4 h-4 shrink-0" />
          {error}
        </div>
      )}

      {result && (
        <div className="rounded-xl bg-emerald-500/10 border border-emerald-500/20 px-4 py-3 space-y-2 text-sm">
          <div className="flex items-center gap-2 text-emerald-400">
            <CheckCircle className="w-4 h-4 shrink-0" />
            Withdrawal instructions received
          </div>
          {result.type && (
            <p className="text-muted-foreground">
              <span className="font-medium text-foreground">Type:</span>{" "}
              {result.type}
            </p>
          )}
          {result.how && (
            <p className="text-muted-foreground">
              <span className="font-medium text-foreground">How:</span>{" "}
              {result.how}
            </p>
          )}
          {result.id && (
            <p className="text-muted-foreground font-mono text-xs">
              Transaction id: {result.id}
            </p>
          )}
          {result.url && (
            <a
              href={result.url}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-accent hover:underline"
            >
              Open instructions <ExternalLink className="w-3 h-3" />
            </a>
          )}
        </div>
      )}

      <button
        type="submit"
        disabled={!canSubmit}
        className="rounded-xl bg-accent text-accent-foreground px-6 py-2.5 font-medium hover:opacity-90 flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {loading ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <ArrowUpFromLine className="w-4 h-4" />
        )}
        Request withdrawal
      </button>
    </form>
  );
}
