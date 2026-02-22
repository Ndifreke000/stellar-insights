"use client";

import React from "react";
import Link from "next/link";
import { Clock, Users } from "lucide-react";
import type { Proposal, ProposalStatus } from "@/types/governance";

const statusConfig: Record<
  ProposalStatus,
  { label: string; className: string }
> = {
  draft: {
    label: "Draft",
    className: "bg-slate-500/10 text-slate-400 border-slate-500/30",
  },
  active: {
    label: "Active",
    className: "bg-accent/10 text-accent border-accent/30",
  },
  passed: {
    label: "Passed",
    className: "bg-emerald-500/10 text-emerald-400 border-emerald-500/30",
  },
  failed: {
    label: "Failed",
    className: "bg-red-500/10 text-red-400 border-red-500/30",
  },
  executed: {
    label: "Executed",
    className: "bg-amber-500/10 text-amber-400 border-amber-500/30",
  },
};

function getTimeRemaining(votingEndsAt: string): string {
  const now = new Date().getTime();
  const end = new Date(votingEndsAt).getTime();
  const diff = end - now;

  if (diff <= 0) return "Ended";

  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));

  if (days > 0) return `${days}d ${hours}h left`;
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
  if (hours > 0) return `${hours}h ${minutes}m left`;
  return `${minutes}m left`;
}

interface ProposalCardProps {
  proposal: Proposal;
}

export function ProposalCard({ proposal }: ProposalCardProps) {
  const status = statusConfig[proposal.status];
  const totalVotes =
    proposal.votesFor + proposal.votesAgainst + proposal.votesAbstain;
  const forPct = totalVotes > 0 ? (proposal.votesFor / totalVotes) * 100 : 0;
  const againstPct =
    totalVotes > 0 ? (proposal.votesAgainst / totalVotes) * 100 : 0;

  return (
    <Link href={`/governance/${proposal.id}`}>
      <div className="glass-card rounded-2xl p-6 border border-border/50 hover:border-accent/30 transition-all duration-300 group cursor-pointer">
        <div className="flex items-start justify-between gap-4 mb-4">
          <h3 className="text-sm font-bold uppercase tracking-widest text-foreground group-hover:text-accent transition-colors line-clamp-2">
            {proposal.title}
          </h3>
          <span
            className={`shrink-0 inline-flex items-center rounded-full border px-2.5 py-0.5 text-[10px] font-mono font-bold uppercase tracking-widest ${status.className}`}
          >
            {status.label}
          </span>
        </div>

        <p className="text-xs font-mono text-muted-foreground line-clamp-2 mb-4">
          {proposal.description}
        </p>

        {/* Vote progress bar */}
        {totalVotes > 0 && (
          <div className="mb-4">
            <div className="h-1.5 w-full rounded-full overflow-hidden bg-white/5 flex">
              <div
                className="h-full bg-emerald-500 transition-all duration-500"
                style={{ width: `${forPct}%` }}
              />
              <div
                className="h-full bg-red-500 transition-all duration-500"
                style={{ width: `${againstPct}%` }}
              />
            </div>
            <div className="flex justify-between mt-1.5">
              <span className="text-[10px] font-mono text-emerald-400">
                {forPct.toFixed(0)}% For
              </span>
              <span className="text-[10px] font-mono text-red-400">
                {againstPct.toFixed(0)}% Against
              </span>
            </div>
          </div>
        )}

        <div className="flex items-center justify-between text-[10px] font-mono text-muted-foreground/50">
          <div className="flex items-center gap-1">
            <Users className="w-3 h-3" />
            <span>{proposal.totalVoters} voters</span>
          </div>
          {proposal.status === "active" && (
            <div className="flex items-center gap-1">
              <Clock className="w-3 h-3" />
              <span>{getTimeRemaining(proposal.votingEndsAt)}</span>
            </div>
          )}
        </div>
      </div>
    </Link>
  );
}
