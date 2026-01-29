"use client";

import { IssuedAsset } from "@/lib/api";
import {
  X,
  ArrowUpRight,
  AlertTriangle,
  TrendingUp,
  Activity,
  ShieldCheck,
} from "lucide-react";
import { useEffect, useRef } from "react";

interface AssetDetailModalProps {
  asset: IssuedAsset | null;
  onClose: () => void;
}

export function AssetDetailModal({ asset, onClose }: AssetDetailModalProps) {
  const modalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };

    if (asset) {
      document.addEventListener("keydown", handleEscape);
      document.body.style.overflow = "hidden";
    }

    return () => {
      document.removeEventListener("keydown", handleEscape);
      document.body.style.overflow = "unset";
    };
  }, [asset, onClose]);

  if (!asset) return null;

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (modalRef.current && !modalRef.current.contains(e.target as Node)) {
      onClose();
    }
  };

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
      maximumFractionDigits: 0,
    }).format(value);
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm transition-opacity"
      onClick={handleBackdropClick}
    >
      <div
        ref={modalRef}
        className="bg-slate-900 border border-slate-800 rounded-2xl w-full max-w-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]"
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-slate-800 flex justify-between items-center bg-slate-950/30">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-indigo-500/10 flex items-center justify-center text-indigo-400 border border-indigo-500/20 font-bold">
              {asset.asset_code.substring(0, 2)}
            </div>
            <div>
              <h2 className="text-xl font-bold text-white">
                {asset.asset_code}
              </h2>
              <p className="text-slate-400 text-sm font-mono truncate max-w-[300px]">
                {asset.issuer}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 text-slate-400 hover:text-white hover:bg-slate-800 rounded-lg transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 overflow-y-auto">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-8">
            {/* Primary Stat Card - Volume */}
            <div className="bg-slate-800/30 border border-slate-700/50 rounded-xl p-5">
              <div className="flex items-center gap-2 text-slate-400 text-sm mb-2">
                <Activity className="w-4 h-4 text-indigo-400" />
                24h Volume
              </div>
              <div className="text-3xl font-bold text-white font-mono">
                {formatCurrency(asset.volume_24h_usd)}
              </div>
            </div>

            {/* Primary Stat Card - Transactions */}
            <div className="bg-slate-800/30 border border-slate-700/50 rounded-xl p-5">
              <div className="flex items-center gap-2 text-slate-400 text-sm mb-2">
                <TrendingUp className="w-4 h-4 text-emerald-400" />
                Total Transactions
              </div>
              <div className="text-3xl font-bold text-white font-mono">
                {asset.total_transactions.toLocaleString()}
              </div>
            </div>
          </div>

          <h3 className="text-md font-semibold text-white mb-4 flex items-center gap-2">
            <ShieldCheck className="w-4 h-4 text-slate-400" />
            Performance Metrics
          </h3>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Success Rate */}
            <div className="col-span-1 bg-slate-950/50 rounded-xl p-4 border border-slate-800">
              <div className="flex justify-between items-center mb-2">
                <span className="text-slate-400 text-sm">Success Rate</span>
                <span className="text-emerald-400 bg-emerald-400/10 px-2 py-0.5 rounded text-xs font-mono">
                  High
                </span>
              </div>
              <div className="flex items-baseline gap-2">
                <span className="text-2xl font-bold text-white">
                  {asset.success_rate.toFixed(1)}%
                </span>
                <span className="text-emerald-400 text-xs flex items-center">
                  <ArrowUpRight className="w-3 h-3 mr-0.5" />
                  Healthy
                </span>
              </div>
              <div className="w-full bg-slate-800 h-1.5 rounded-full mt-3 overflow-hidden">
                <div
                  className="bg-emerald-500 h-full rounded-full"
                  style={{ width: `${asset.success_rate}%` }}
                ></div>
              </div>
            </div>

            {/* Failure Rate */}
            <div className="col-span-1 bg-slate-950/50 rounded-xl p-4 border border-slate-800">
              <div className="flex justify-between items-center mb-2">
                <span className="text-slate-400 text-sm">Failure Rate</span>
                {asset.failure_rate > 5 && (
                  <span className="text-rose-400 bg-rose-400/10 px-2 py-0.5 rounded text-xs font-mono flex items-center gap-1">
                    <AlertTriangle className="w-3 h-3" /> Warning
                  </span>
                )}
              </div>
              <div className="flex items-baseline gap-2">
                <span
                  className={`text-2xl font-bold ${asset.failure_rate > 5 ? "text-rose-400" : "text-slate-300"}`}
                >
                  {asset.failure_rate.toFixed(1)}%
                </span>
              </div>
              <div className="w-full bg-slate-800 h-1.5 rounded-full mt-3 overflow-hidden">
                <div
                  className={`h-full rounded-full ${asset.failure_rate > 5 ? "bg-rose-500" : "bg-slate-500"}`}
                  style={{ width: `${Math.min(asset.failure_rate * 5, 100)}%` }}
                ></div>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-slate-800 bg-slate-950/30 flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-white rounded-lg transition-colors text-sm font-medium"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
