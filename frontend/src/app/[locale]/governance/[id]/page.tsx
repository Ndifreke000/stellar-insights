"use client";

import React, { useState, useEffect, useCallback } from "react";
import { useParams } from "next/navigation";
import { getProposal } from "@/lib/governance-api";
import { ProposalDetail } from "@/components/governance/ProposalDetail";
import type { Proposal } from "@/types/governance";

export default function GovernanceProposalPage() {
  const params = useParams();
  const id = params.id as string;
  const [proposal, setProposal] = useState<Proposal | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchProposal = useCallback(async () => {
    try {
      const data = await getProposal(id);
      setProposal(data);
      setError(null);
    } catch (err) {
      const msg =
        err instanceof Error ? err.message : "Failed to load proposal";
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    fetchProposal();
  }, [fetchProposal]);

  if (loading) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest">
          Loading Proposal... // Fetching Details
        </div>
      </div>
    );
  }

  if (error || !proposal) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="px-6 py-4 glass border-red-500/50 text-red-500 font-mono text-sm uppercase tracking-widest">
          Error: {error || "Proposal not found"}
        </div>
      </div>
    );
  }

  return (
    <div className="animate-in fade-in slide-in-from-bottom-4 duration-700">
      <ProposalDetail proposal={proposal} onVoted={fetchProposal} />
    </div>
  );
}
