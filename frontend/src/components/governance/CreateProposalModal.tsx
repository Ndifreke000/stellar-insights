"use client";

import React, { useState } from "react";
import { X } from "lucide-react";
import { createProposal } from "@/lib/governance-api";
import type { CreateProposalRequest } from "@/types/governance";

interface CreateProposalModalProps {
  authToken: string;
  onClose: () => void;
  onCreated: () => void;
}

export function CreateProposalModal({
  authToken,
  onClose,
  onCreated,
}: CreateProposalModalProps) {
  const [form, setForm] = useState<CreateProposalRequest>({
    title: "",
    description: "",
    targetContract: "",
    newWasmHash: "",
  });
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    setError(null);

    try {
      await createProposal(form, authToken);
      onCreated();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create proposal");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative glass-card rounded-2xl border border-border/50 w-full max-w-lg mx-4 p-6 animate-in fade-in zoom-in-95 duration-200">
        <div className="flex items-center justify-between mb-6">
          <h3 className="text-sm font-bold uppercase tracking-widest text-foreground">
            Create Proposal
          </h3>
          <button
            onClick={onClose}
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {error && (
          <div className="mb-4 px-4 py-2 rounded-xl border border-red-500/30 bg-red-500/10 text-red-400 text-xs font-mono">
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-2">
              Title
            </label>
            <input
              type="text"
              required
              value={form.title}
              onChange={(e) => setForm({ ...form, title: e.target.value })}
              className="w-full bg-white/5 border border-border/50 rounded-xl px-4 py-3 text-sm font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:border-accent/50 transition-colors"
              placeholder="Proposal title..."
            />
          </div>

          <div>
            <label className="block text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-2">
              Description
            </label>
            <textarea
              required
              rows={4}
              value={form.description}
              onChange={(e) =>
                setForm({ ...form, description: e.target.value })
              }
              className="w-full bg-white/5 border border-border/50 rounded-xl px-4 py-3 text-sm font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:border-accent/50 transition-colors resize-none"
              placeholder="Describe the proposal..."
            />
          </div>

          <div>
            <label className="block text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-2">
              Target Contract
            </label>
            <input
              type="text"
              required
              value={form.targetContract}
              onChange={(e) =>
                setForm({ ...form, targetContract: e.target.value })
              }
              className="w-full bg-white/5 border border-border/50 rounded-xl px-4 py-3 text-sm font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:border-accent/50 transition-colors"
              placeholder="C..."
            />
          </div>

          <div>
            <label className="block text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-2">
              New WASM Hash
            </label>
            <input
              type="text"
              required
              value={form.newWasmHash}
              onChange={(e) =>
                setForm({ ...form, newWasmHash: e.target.value })
              }
              className="w-full bg-white/5 border border-border/50 rounded-xl px-4 py-3 text-sm font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:border-accent/50 transition-colors"
              placeholder="WASM hash..."
            />
          </div>

          <div className="flex items-center justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-xl text-xs font-bold uppercase tracking-widest text-muted-foreground hover:text-foreground transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={submitting}
              className="px-6 py-2 rounded-xl bg-accent/10 border border-accent/30 text-accent text-xs font-bold uppercase tracking-widest hover:bg-accent/20 transition-colors disabled:opacity-50"
            >
              {submitting ? "Submitting..." : "Create Proposal"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
