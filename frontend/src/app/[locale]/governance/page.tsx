"use client";

import React, { useState, useEffect, useCallback } from "react";
import { ScrollText, Plus } from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { ProposalCard } from "@/components/governance/ProposalCard";
import { CreateProposalModal } from "@/components/governance/CreateProposalModal";
import { useWallet } from "@/components/lib/wallet-context";
import { getProposals } from "@/lib/governance-api";
import type { Proposal, ProposalStatus } from "@/types/governance";

const STATUS_TABS: { label: string; value: ProposalStatus | "all" }[] = [
  { label: "All", value: "all" },
  { label: "Active", value: "active" },
  { label: "Passed", value: "passed" },
  { label: "Failed", value: "failed" },
  { label: "Executed", value: "executed" },
];

export default function GovernancePage() {
  const { isAuthenticated, authToken } = useWallet();
  const [proposals, setProposals] = useState<Proposal[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<ProposalStatus | "all">("all");
  const [showCreateModal, setShowCreateModal] = useState(false);

  const fetchData = useCallback(async () => {
    try {
      const statusFilter = activeTab === "all" ? undefined : activeTab;
      const data = await getProposals(statusFilter);
      setProposals(data.proposals);
      setTotal(data.total);
      setError(null);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load proposals";
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, [activeTab]);

  useEffect(() => {
    setLoading(true);
    fetchData();
  }, [fetchData]);

  const activeCount = proposals.filter((p) => p.status === "active").length;
  const passedCount = proposals.filter((p) => p.status === "passed").length;
  const failedCount = proposals.filter((p) => p.status === "failed").length;
  const totalVotes = proposals.reduce(
    (sum, p) => sum + p.votesFor + p.votesAgainst + p.votesAbstain,
    0,
  );

  if (loading) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest">
          Loading Governance Data... // Querying Proposals
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
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      {/* Page Header */}
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
            onClick={() => setShowCreateModal(true)}
            className="flex items-center gap-2 px-4 py-2 rounded-xl bg-accent/10 border border-accent/30 text-accent text-xs font-bold uppercase tracking-widest hover:bg-accent/20 transition-colors"
          >
            <Plus className="w-4 h-4" />
            Create Proposal
          </button>
        )}
      </div>

      {/* Metric Cards */}
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

      {/* Status Filter Tabs */}
      <div className="flex items-center gap-2 overflow-x-auto pb-2">
        {STATUS_TABS.map((tab) => (
          <button
            key={tab.value}
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

      {/* Proposals Grid */}
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

      {/* Create Proposal Modal */}
      {showCreateModal && authToken && (
        <CreateProposalModal
          authToken={authToken}
          onClose={() => setShowCreateModal(false)}
          onCreated={fetchData}
        />
      )}
    </div>
  );
}
