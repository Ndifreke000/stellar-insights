"use client";

import React, { useState, useEffect, useRef } from "react";
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
  const modalRef = useRef<HTMLDivElement>(null);

  // Focus trap and keyboard handling
  useEffect(() => {
    const focusableElements = modalRef.current?.querySelectorAll(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    
    const firstElement = focusableElements?.[0] as HTMLElement;
    const lastElement = focusableElements?.[focusableElements.length - 1] as HTMLElement;
    
    firstElement?.focus();
    
    const handleTab = (e: KeyboardEvent) => {
      if (e.key === 'Tab') {
        if (e.shiftKey && document.activeElement === firstElement) {
          e.preventDefault();
          lastElement?.focus();
        } else if (!e.shiftKey && document.activeElement === lastElement) {
          e.preventDefault();
          firstElement?.focus();
        }
      }
      
      if (e.key === 'Escape') {
        onClose();
      }
    };
    
    document.addEventListener('keydown', handleTab);
    return () => document.removeEventListener('keydown', handleTab);
  }, [onClose]);

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
    <div 
      className="fixed inset-0 z-50 flex items-center justify-center"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
      aria-describedby="modal-description"
    >
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Modal */}
      <div 
        ref={modalRef}
        className="relative glass-card rounded-2xl border border-border/50 w-full max-w-lg mx-4 p-6 animate-in fade-in zoom-in-95 duration-200"
      >
        <div className="flex items-center justify-between mb-6">
          <h3 
            id="modal-title"
            className="text-sm font-bold uppercase tracking-widest text-foreground"
          >
            Create Proposal
          </h3>
          <button
            onClick={onClose}
            aria-label="Close modal"
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="w-5 h-5" aria-hidden="true" />
          </button>
        </div>

        {error && (
          <div 
            role="alert"
            aria-live="assertive"
            className="mb-4 px-4 py-2 rounded-xl border border-red-500/30 bg-red-500/10 text-red-400 text-xs font-mono"
          >
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4" id="modal-description">
          <div>
            <label 
              htmlFor="proposal-title"
              className="block text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-2"
            >
              Title
            </label>
            <input
              id="proposal-title"
              type="text"
              required
              value={form.title}
              onChange={(e) => setForm({ ...form, title: e.target.value })}
              className="w-full bg-white/5 border border-border/50 rounded-xl px-4 py-3 text-sm font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:border-accent/50 transition-colors"
              placeholder="Proposal title..."
              aria-required="true"
            />
          </div>

          <div>
            <label 
              htmlFor="proposal-description"
              className="block text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-2"
            >
              Description
            </label>
            <textarea
              id="proposal-description"
              required
              rows={4}
              value={form.description}
              onChange={(e) =>
                setForm({ ...form, description: e.target.value })
              }
              className="w-full bg-white/5 border border-border/50 rounded-xl px-4 py-3 text-sm font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:border-accent/50 transition-colors resize-none"
              placeholder="Describe the proposal..."
              aria-required="true"
            />
          </div>

          <div>
            <label 
              htmlFor="target-contract"
              className="block text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-2"
            >
              Target Contract
            </label>
            <input
              id="target-contract"
              type="text"
              required
              value={form.targetContract}
              onChange={(e) =>
                setForm({ ...form, targetContract: e.target.value })
              }
              className="w-full bg-white/5 border border-border/50 rounded-xl px-4 py-3 text-sm font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:border-accent/50 transition-colors"
              placeholder="C..."
              aria-required="true"
            />
          </div>

          <div>
            <label 
              htmlFor="wasm-hash"
              className="block text-[10px] font-mono font-bold uppercase tracking-widest text-muted-foreground mb-2"
            >
              New WASM Hash
            </label>
            <input
              id="wasm-hash"
              type="text"
              required
              value={form.newWasmHash}
              onChange={(e) =>
                setForm({ ...form, newWasmHash: e.target.value })
              }
              className="w-full bg-white/5 border border-border/50 rounded-xl px-4 py-3 text-sm font-mono text-foreground placeholder:text-muted-foreground/30 focus:outline-none focus:border-accent/50 transition-colors"
              placeholder="WASM hash..."
              aria-required="true"
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
