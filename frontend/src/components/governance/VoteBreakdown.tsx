"use client";

import React from "react";

interface VoteBreakdownProps {
  votesFor: number;
  votesAgainst: number;
  votesAbstain: number;
  totalVoters: number;
}

export function VoteBreakdown({
  votesFor,
  votesAgainst,
  votesAbstain,
  totalVoters,
}: VoteBreakdownProps) {
  const total = votesFor + votesAgainst + votesAbstain;
  const forPct = total > 0 ? (votesFor / total) * 100 : 0;
  const againstPct = total > 0 ? (votesAgainst / total) * 100 : 0;
  const abstainPct = total > 0 ? (votesAbstain / total) * 100 : 0;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
        <span>Vote Breakdown</span>
        <span>{totalVoters} voter{totalVoters !== 1 ? "s" : ""}</span>
      </div>

      {/* Horizontal bar */}
      <div className="h-3 w-full rounded-full overflow-hidden bg-white/5 flex">
        {forPct > 0 && (
          <div
            className="h-full bg-emerald-500 transition-all duration-500"
            style={{ width: `${forPct}%` }}
          />
        )}
        {againstPct > 0 && (
          <div
            className="h-full bg-red-500 transition-all duration-500"
            style={{ width: `${againstPct}%` }}
          />
        )}
        {abstainPct > 0 && (
          <div
            className="h-full bg-slate-500 transition-all duration-500"
            style={{ width: `${abstainPct}%` }}
          />
        )}
      </div>

      {/* Legend */}
      <div className="grid grid-cols-3 gap-4">
        <div>
          <div className="flex items-center gap-2 mb-1">
            <div className="w-2 h-2 rounded-full bg-emerald-500" />
            <span className="text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
              For
            </span>
          </div>
          <div className="text-lg font-mono font-bold text-foreground">
            {votesFor}
          </div>
          <div className="text-[10px] font-mono text-muted-foreground/50">
            {forPct.toFixed(1)}%
          </div>
        </div>
        <div>
          <div className="flex items-center gap-2 mb-1">
            <div className="w-2 h-2 rounded-full bg-red-500" />
            <span className="text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
              Against
            </span>
          </div>
          <div className="text-lg font-mono font-bold text-foreground">
            {votesAgainst}
          </div>
          <div className="text-[10px] font-mono text-muted-foreground/50">
            {againstPct.toFixed(1)}%
          </div>
        </div>
        <div>
          <div className="flex items-center gap-2 mb-1">
            <div className="w-2 h-2 rounded-full bg-slate-500" />
            <span className="text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
              Abstain
            </span>
          </div>
          <div className="text-lg font-mono font-bold text-foreground">
            {votesAbstain}
          </div>
          <div className="text-[10px] font-mono text-muted-foreground/50">
            {abstainPct.toFixed(1)}%
          </div>
        </div>
      </div>
    </div>
  );
}
