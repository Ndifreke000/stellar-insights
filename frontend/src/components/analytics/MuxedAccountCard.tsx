"use client";

import React, { useEffect, useState } from "react";
import { Users, Activity } from "lucide-react";
import { getMuxedAnalytics, MuxedAccountAnalytics } from "@/lib/api";
import { formatAddressShort } from "@/lib/address";

export function MuxedAccountCard() {
  const [data, setData] = useState<MuxedAccountAnalytics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getMuxedAnalytics(10)
      .then((res) => {
        if (!cancelled) setData(res);
      })
      .catch(() => {
        if (!cancelled) setError("Failed to load muxed analytics");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return (
      <div className="glass-card rounded-2xl p-6 animate-pulse">
        <div className="h-5 w-32 bg-white/10 rounded mb-4" />
        <div className="h-20 bg-white/10 rounded mb-4" />
        <div className="h-24 bg-white/10 rounded" />
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="glass-card rounded-2xl p-6">
        <div className="text-[10px] font-mono text-accent uppercase tracking-widest mb-2">
          Muxed Accounts (M-addresses)
        </div>
        <p className="text-sm text-muted-foreground">
          {error || "No muxed account data available."}
        </p>
      </div>
    );
  }

  const hasAny = data.total_muxed_payments > 0 || data.unique_muxed_addresses > 0;

  return (
    <div className="glass-card rounded-2xl p-6">
      <div className="text-[10px] font-mono text-accent uppercase tracking-widest mb-3">
        Muxed Accounts (M-addresses)
      </div>
      <p className="text-xs text-muted-foreground mb-4">
        Sub-accounts (M...) usage and activity across payments.
      </p>
      <div className="grid grid-cols-2 gap-3 mb-4">
        <div className="bg-white/5 rounded-lg p-3 flex items-center gap-2">
          <Activity className="w-4 h-4 text-accent" />
          <div>
            <div className="text-lg font-bold tabular-nums">
              {data.total_muxed_payments}
            </div>
            <div className="text-[10px] text-muted-foreground uppercase tracking-wider">
              Muxed payments
            </div>
          </div>
        </div>
        <div className="bg-white/5 rounded-lg p-3 flex items-center gap-2">
          <Users className="w-4 h-4 text-accent" />
          <div>
            <div className="text-lg font-bold tabular-nums">
              {data.unique_muxed_addresses}
            </div>
            <div className="text-[10px] text-muted-foreground uppercase tracking-wider">
              Unique M-addresses
            </div>
          </div>
        </div>
      </div>
      {hasAny && data.top_muxed_by_activity.length > 0 && (
        <div>
          <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider mb-2">
            Top by activity
          </div>
          <ul className="space-y-2 max-h-48 overflow-y-auto">
            {data.top_muxed_by_activity.map((u) => (
              <li
                key={u.account_address}
                className="flex items-center justify-between gap-2 text-xs bg-white/5 rounded-lg px-3 py-2"
              >
                <div className="min-w-0 flex-1">
                  <span
                    className="font-mono text-foreground truncate block"
                    title={u.account_address}
                  >
                    {formatAddressShort(u.account_address, 6, 6)}
                  </span>
                  {u.base_account && (
                    <span className="text-[10px] text-muted-foreground">
                      Base: {formatAddressShort(u.base_account, 4, 4)}
                      {u.muxed_id != null && ` · ID ${u.muxed_id}`}
                    </span>
                  )}
                </div>
                <div className="tabular-nums shrink-0 text-muted-foreground">
                  {u.total_payments} tx
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}
      {hasAny && data.base_accounts_with_muxed.length > 0 && (
        <div className="mt-3 pt-3 border-t border-white/10">
          <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider mb-1">
            Base accounts with muxed sub-accounts
          </div>
          <p className="text-xs text-muted-foreground">
            {data.base_accounts_with_muxed.length} G-address(es)
          </p>
        </div>
      )}
    </div>
  );
}
