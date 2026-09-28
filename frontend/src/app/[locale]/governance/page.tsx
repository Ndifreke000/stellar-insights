"use client";

import { useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import dynamic from "next/dynamic";
import { ScrollText, Plus } from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { ProposalCard } from "@/components/governance/ProposalCard";
import { useWallet } from "@/components/lib/wallet-context";
import { useProposals } from "@/lib/react-query/queries";
import { queryKeys } from "@/lib/react-query/keys";
import type { ProposalStatus } from "@/types/governance";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { SkeletonCard } from "@/components/ui/Skeleton";

// Lazy-load the modal — it's only needed when the user clicks "Create Proposal".
const CreateProposalModal = dynamic(
  () => import("@/components/governance/CreateProposalModal").then((m) => ({ default: m.CreateProposalModal })),
  { ssr: false }
);

const STATUS_TABS: { label: string; value: ProposalStatus | "all" }[] = [
  { label: "All", value: "all" },
  { label: "Active", value: "active" },
  { label: "Passed", value: "passed" },
  { label: "Failed", value: "failed" },
  { label: "Executed", value: "executed" },
];

export default function GovernancePage() {
  const { isAuthenticated, authToken } = useWallet();
  const [activeTab, setActiveTab] = useState<ProposalStatus | "all">("all");
  const [showCreateModal, setShowCreateModal] = useState(false);

  const queryClient = useQueryClient();
  const proposalsQuery = useProposals(activeTab === "all" ? undefined : activeTab);
  const proposals = useMemo(
    () => proposalsQuery.data?.data ?? [],
    [proposalsQuery.data],
  );
  const total = proposalsQuery.data?.pagination.total ?? proposals.length;
  const loading = proposalsQuery.isPending;
  const error = proposalsQuery.isError
    ? proposalsQuery.error.message || "Failed to load proposals"
    : null;
  // A new proposal can appear under several status tabs — refresh them all.
  const refreshProposals = () =>
    queryClient.invalidateQueries({ queryKey: queryKeys.governance });

  const activeCount = proposals.filter((p) => p.status === "active").length;
  const passedCount = proposals.filter((p) => p.status === "passed").length;
  const failedCount = proposals.filter((p) => p.status === "failed").length;
  const totalVotes = proposals.reduce(
    (sum, p) => sum + p.votesFor + p.votesAgainst + p.votesAbstain,
    0,
  );

  if (loading) {
    return (
      <div className="space-y-8 animate-in fade-in duration-500">
        {/* Header skeleton */}
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
          <div className="space-y-2">
            <div className="h-3 w-32 bg-accent/20 rounded animate-pulse" />
            <div className="h-9 w-48 bg-white/10 rounded animate-pulse" />
          </div>
        </div>
        {/* Metric cards skeleton */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {[1, 2, 3, 4].map((i) => (
            <SkeletonCard key={i} />
          ))}
        </div>
        {/* Tab row skeleton */}
        <div className="flex items-center gap-2">
          {[1, 2, 3, 4, 5].map((i) => (
            <div key={i} className="h-8 w-16 bg-white/5 border border-border/20 rounded-xl animate-pulse" />
          ))}
        </div>
        {/* Proposal cards skeleton */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="glass-card rounded-2xl p-6 border border-border/50 space-y-4 animate-pulse">
              <div className="flex items-start justify-between">
                <div className="space-y-2 flex-1">
                  <div className="h-5 w-3/4 bg-white/10 rounded" />
                  <div className="h-3 w-1/2 bg-white/5 rounded" />
                </div>
                <div className="h-6 w-16 bg-accent/20 rounded-full" />
              </div>
              <div className="h-3 w-full bg-white/5 rounded" />
              <div className="h-3 w-4/5 bg-white/5 rounded" />
              <div className="h-2 w-full bg-white/5 rounded-full" />
              <div className="flex gap-2">
                <div className="h-8 flex-1 bg-white/5 rounded-lg" />
                <div className="h-8 flex-1 bg-white/5 rounded-lg" />
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="px-6 py-4 glass border-red-500/50 text-red-500 font-mono text-sm uppercase tracking-widest">
          Governance Error: {error}
        </div>
      </div>
    );
  }

  return (
    <ErrorBoundary>
      <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
          <div>
            <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
              Governance // Proposals
            </div>
            <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
              <ScrollText className="w-8 h-8 text-accent" />
              Governance
            </h2>
          </div>
          {isAuthenticated && authToken && (
            <button
              type="button"
              onClick={() => setShowCreateModal(true)}
              className="flex items-center gap-2 px-4 py-2 rounded-xl bg-accent/10 border border-accent/30 text-accent text-xs font-bold uppercase tracking-widest hover:bg-accent/20 transition-colors"
            >
              <Plus className="w-4 h-4" />
              Create Proposal
            </button>
          )}
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <MetricCard
            label="Active Proposals"
            value={activeCount}
            subLabel="Currently voting"
          />
          <MetricCard
            label="Total Votes"
            value={totalVotes}
            subLabel="Across all proposals"
          />
          <MetricCard
            label="Passed"
            value={passedCount}
            subLabel="Approved proposals"
          />
          <MetricCard
            label="Failed"
            value={failedCount}
            subLabel="Rejected proposals"
          />
        </div>

        <div className="flex items-center gap-2 overflow-x-auto pb-2">
          {STATUS_TABS.map((tab) => (
            <button
              key={tab.value}
              type="button"
              aria-current={activeTab === tab.value ? "true" : undefined}
              onClick={() => setActiveTab(tab.value)}
              className={`px-4 py-2 rounded-xl text-[10px] font-bold uppercase tracking-widest transition-all duration-300 whitespace-nowrap ${
                activeTab === tab.value
                  ? "bg-accent/10 text-accent border border-accent/30"
                  : "text-muted-foreground hover:text-foreground hover:bg-white/5 border border-transparent"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {proposals.length === 0 ? (
          <div className="glass-card rounded-2xl p-12 border border-border/50 text-center">
            <p className="text-xs font-mono text-muted-foreground/50 uppercase tracking-widest">
              No proposals found
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {proposals.map((proposal) => (
              <ProposalCard key={proposal.id} proposal={proposal} />
            ))}
          </div>
        )}

        {showCreateModal && authToken && (
          <CreateProposalModal
            authToken={authToken}
            onClose={() => setShowCreateModal(false)}
            onCreated={refreshProposals}
          />
        )}
      </div>
    </ErrorBoundary>
  );
}