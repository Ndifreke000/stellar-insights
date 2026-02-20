"use client";

import React from "react";
import { AlertTriangle, Clock, Users, Wallet } from "lucide-react";
import type { ClaimableBalance } from "@/lib/claimable-balance-api";

interface ClaimableBalanceCardProps {
  balance: ClaimableBalance;
}

function formatAddress(addr: string) {
  if (!addr || addr.length < 12) return addr;
  return `${addr.slice(0, 6)}...${addr.slice(-6)}`;
}

function formatExpiry(expiresAt: string | null): string {
  if (!expiresAt) return "Never";
  const exp = new Date(expiresAt);
  const now = new Date();
  const diffMs = exp.getTime() - now.getTime();
  const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));
  if (diffDays < 0) return "Expired";
  if (diffDays === 0) return "Expires today";
  if (diffDays === 1) return "1 day";
  return `${diffDays} days`;
}

function isExpiringSoon(expiresAt: string | null, daysThreshold: number = 10): boolean {
  if (!expiresAt) return false;
  const exp = new Date(expiresAt);
  const now = new Date();
  const diffMs = exp.getTime() - now.getTime();
  const diffDays = diffMs / (1000 * 60 * 60 * 24);
  return diffDays > 0 && diffDays <= daysThreshold;
}

export function ClaimableBalanceCard({ balance }: ClaimableBalanceCardProps) {
  const expiringSoon = isExpiringSoon(balance.expires_at);
  const claimantCount = balance.claimant_count ?? 0;

  return (
    <div
      className={`rounded-2xl border p-6 transition-all duration-200 hover:scale-[1.01] ${
        expiringSoon
          ? "border-amber-500/40 bg-amber-500/5"
          : "border-border/50 bg-slate-900/30 hover:border-accent/30"
      }`}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="space-y-2">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-accent/20">
              <Wallet className="h-5 w-5 text-accent" />
            </div>
            <div>
              <div className="text-xl font-black font-mono text-foreground">
                {Number(balance.amount).toLocaleString()} {balance.asset_code}
              </div>
              <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider">
                {balance.asset_issuer ? `${balance.asset_code}:${formatAddress(balance.asset_issuer)}` : "Native XLM"}
              </div>
            </div>
          </div>

          <div className="flex flex-wrap gap-3 pt-2">
            <div className="flex items-center gap-1.5 text-[10px] font-mono text-muted-foreground">
              <Users className="h-3 w-3" />
              <span>Sponsor: {formatAddress(balance.sponsor)}</span>
            </div>
            <div className="flex items-center gap-1.5 text-[10px] font-mono text-muted-foreground">
              <Users className="h-3 w-3" />
              <span>{claimantCount} claimant{claimantCount !== 1 ? "s" : ""}</span>
            </div>
          </div>
        </div>

        <div className="flex flex-col items-end gap-2">
          <div
            className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-[10px] font-mono font-bold uppercase ${
              expiringSoon
                ? "bg-amber-500/20 text-amber-400"
                : "bg-slate-800/50 text-muted-foreground"
            }`}
          >
            {expiringSoon && <AlertTriangle className="h-3 w-3" />}
            <Clock className="h-3 w-3" />
            <span>
              {balance.expires_at
                ? `Expires: ${formatExpiry(balance.expires_at)}`
                : "No expiration"}
            </span>
          </div>
          {balance.expires_at && (
            <div className="text-[9px] font-mono text-muted-foreground/70">
              {new Date(balance.expires_at).toLocaleDateString()}
            </div>
          )}
        </div>
      </div>

      <div className="mt-4 border-t border-border/30 pt-3">
        <div className="text-[9px] font-mono text-muted-foreground/50 break-all">
          ID: {balance.id}
        </div>
      </div>
    </div>
  );
}
