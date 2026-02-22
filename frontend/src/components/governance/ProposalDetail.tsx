"use client";

import React, { useState, useEffect, useCallback } from "react";
import { ArrowLeft, CheckCircle, XCircle, MinusCircle, Send } from "lucide-react";
import Link from "next/link";
import { useWallet } from "@/components/lib/wallet-context";
import { VoteBreakdown } from "./VoteBreakdown";
import {
  castVote,
  hasVoted,
  getComments,
  addComment,
} from "@/lib/governance-api";
import type { Proposal, Comment, VoteChoice } from "@/types/governance";

const statusConfig: Record<
  string,
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

interface ProposalDetailProps {
  proposal: Proposal;
  onVoted: () => void;
}

export function ProposalDetail({ proposal, onVoted }: ProposalDetailProps) {
  const { isAuthenticated, authToken, address } = useWallet();
  const [userVote, setUserVote] = useState<string | null>(null);
  const [voting, setVoting] = useState(false);
  const [comments, setComments] = useState<Comment[]>([]);
  const [newComment, setNewComment] = useState("");
  const [submittingComment, setSubmittingComment] = useState(false);

  const status = statusConfig[proposal.status] ?? statusConfig.draft;

  const fetchComments = useCallback(async () => {
    try {
      const data = await getComments(proposal.id);
      setComments(data);
    } catch {
      // Silently fail for comments
    }
  }, [proposal.id]);

  useEffect(() => {
    fetchComments();
  }, [fetchComments]);

  useEffect(() => {
    if (address) {
      hasVoted(proposal.id, address)
        .then((result) => {
          if (result.has_voted) {
            setUserVote("voted");
          }
        })
        .catch(() => {});
    }
  }, [proposal.id, address]);

  const handleVote = async (choice: VoteChoice) => {
    if (!authToken) return;
    setVoting(true);
    try {
      await castVote(proposal.id, { choice }, authToken);
      setUserVote(choice);
      onVoted();
    } catch (err) {
      console.error("Vote failed:", err);
    } finally {
      setVoting(false);
    }
  };

  const handleComment = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!authToken || !newComment.trim()) return;
    setSubmittingComment(true);
    try {
      await addComment(proposal.id, newComment.trim(), authToken);
      setNewComment("");
      fetchComments();
    } catch (err) {
      console.error("Comment failed:", err);
    } finally {
      setSubmittingComment(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Back link */}
      <Link
        href="/governance"
        className="inline-flex items-center gap-2 text-xs font-mono text-muted-foreground hover:text-accent transition-colors uppercase tracking-widest"
      >
        <ArrowLeft className="w-4 h-4" />
        Back to Proposals
      </Link>

      {/* Header */}
      <div className="glass-card rounded-2xl p-6 border border-border/50">
        <div className="flex items-start justify-between gap-4 mb-4">
          <h1 className="text-2xl font-black tracking-tighter uppercase italic text-foreground">
            {proposal.title}
          </h1>
          <span
            className={`shrink-0 inline-flex items-center rounded-full border px-3 py-1 text-[10px] font-mono font-bold uppercase tracking-widest ${status.className}`}
          >
            {status.label}
          </span>
        </div>

        <div className="flex flex-wrap gap-4 text-[10px] font-mono text-muted-foreground/50 uppercase tracking-widest mb-6">
          <span>
            By {proposal.createdBy.slice(0, 8)}...{proposal.createdBy.slice(-4)}
          </span>
          <span>
            Created {new Date(proposal.createdAt).toLocaleDateString()}
          </span>
          {proposal.votingEndsAt && (
            <span>
              Voting ends{" "}
              {new Date(proposal.votingEndsAt).toLocaleDateString()}
            </span>
          )}
        </div>

        <div className="text-sm font-mono text-muted-foreground leading-relaxed whitespace-pre-wrap">
          {proposal.description}
        </div>

        {proposal.targetContract && (
          <div className="mt-4 pt-4 border-t border-border/30">
            <div className="text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-1">
              Target Contract
            </div>
            <div className="text-xs font-mono text-foreground break-all">
              {proposal.targetContract}
            </div>
          </div>
        )}

        {proposal.newWasmHash && (
          <div className="mt-3">
            <div className="text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-1">
              WASM Hash
            </div>
            <div className="text-xs font-mono text-foreground break-all">
              {proposal.newWasmHash}
            </div>
          </div>
        )}
      </div>

      {/* Vote Breakdown */}
      <div className="glass-card rounded-2xl p-6 border border-border/50">
        <VoteBreakdown
          votesFor={proposal.votesFor}
          votesAgainst={proposal.votesAgainst}
          votesAbstain={proposal.votesAbstain}
          totalVoters={proposal.totalVoters}
        />

        {/* Vote buttons */}
        {proposal.status === "active" && isAuthenticated && !userVote && (
          <div className="mt-6 pt-4 border-t border-border/30">
            <div className="text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-3">
              Cast Your Vote
            </div>
            <div className="flex flex-wrap gap-3">
              <button
                onClick={() => handleVote("for")}
                disabled={voting}
                className="flex items-center gap-2 px-4 py-2 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 text-xs font-bold uppercase tracking-widest hover:bg-emerald-500/20 transition-colors disabled:opacity-50"
              >
                <CheckCircle className="w-4 h-4" />
                Vote For
              </button>
              <button
                onClick={() => handleVote("against")}
                disabled={voting}
                className="flex items-center gap-2 px-4 py-2 rounded-xl bg-red-500/10 border border-red-500/30 text-red-400 text-xs font-bold uppercase tracking-widest hover:bg-red-500/20 transition-colors disabled:opacity-50"
              >
                <XCircle className="w-4 h-4" />
                Vote Against
              </button>
              <button
                onClick={() => handleVote("abstain")}
                disabled={voting}
                className="flex items-center gap-2 px-4 py-2 rounded-xl bg-slate-500/10 border border-slate-500/30 text-slate-400 text-xs font-bold uppercase tracking-widest hover:bg-slate-500/20 transition-colors disabled:opacity-50"
              >
                <MinusCircle className="w-4 h-4" />
                Abstain
              </button>
            </div>
          </div>
        )}

        {userVote && (
          <div className="mt-6 pt-4 border-t border-border/30">
            <div className="text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
              You voted{" "}
              <span
                className={
                  userVote === "for"
                    ? "text-emerald-400 font-bold"
                    : userVote === "against"
                      ? "text-red-400 font-bold"
                      : "text-slate-400 font-bold"
                }
              >
                {userVote}
              </span>
            </div>
          </div>
        )}
      </div>

      {/* Comments */}
      <div className="glass-card rounded-2xl p-6 border border-border/50">
        <h3 className="text-sm font-mono font-bold uppercase tracking-widest text-foreground mb-4">
          Discussion
        </h3>

        {comments.length === 0 && (
          <p className="text-xs font-mono text-muted-foreground/50 uppercase tracking-widest">
            No comments yet
          </p>
        )}

        <div className="space-y-4">
          {comments.map((comment) => (
            <div
              key={comment.id}
              className="border-b border-border/20 pb-4 last:border-0"
            >
              <div className="flex items-center gap-2 mb-2">
                <span className="text-[10px] font-mono text-accent font-bold">
                  {comment.authorAddress.slice(0, 8)}...
                  {comment.authorAddress.slice(-4)}
                </span>
                <span className="text-[10px] font-mono text-muted-foreground/30">
                  {new Date(comment.createdAt).toLocaleString()}
                </span>
              </div>
              <p className="text-xs font-mono text-muted-foreground">
                {comment.content}
              </p>
            </div>
          ))}
        </div>

        {isAuthenticated && (
          <form onSubmit={handleComment} className="mt-4 flex gap-3">
            <input
              type="text"
              value={newComment}
              onChange={(e) => setNewComment(e.target.value)}
              placeholder="Add a comment..."
              className="flex-1 bg-white/5 border border-border/50 rounded-xl px-4 py-2 text-xs font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:border-accent/50 transition-colors"
            />
            <button
              type="submit"
              disabled={submittingComment || !newComment.trim()}
              className="px-4 py-2 rounded-xl bg-accent/10 border border-accent/30 text-accent hover:bg-accent/20 transition-colors disabled:opacity-50"
            >
              <Send className="w-4 h-4" />
            </button>
          </form>
        )}
      </div>
    </div>
  );
}
