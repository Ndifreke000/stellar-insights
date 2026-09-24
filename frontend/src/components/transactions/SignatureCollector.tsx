'use client';

import React, { useState, useCallback } from 'react';

export interface Signature {
  signer: string;
  signature: string;
}

export interface SignatureCollectorProps {
  transactionId: string;
  xdr: string;
  requiredSignatures: number;
  onSignatureAdded: (txId: string, sig: Signature) => Promise<void> | void;
  onSubmitTransaction: (txId: string) => Promise<void> | void;
}

/**
 * SignatureCollector — Multi-signature collection UI for Stellar transactions.
 * Issue #1839: Implements the signer list, signature collection, and
 * threshold-based submission that was previously a maintenance placeholder.
 *
 * Flow:
 * 1. Displays the list of collected signatures and how many more are needed.
 * 2. Each signer enters their signature (or signs via wallet).
 * 3. Once requiredSignatures is met, the Submit button becomes active.
 * 4. On submit, calls onSubmitTransaction(txId) to broadcast the fully-signed tx.
 */
export function SignatureCollector({
  transactionId,
  xdr,
  requiredSignatures,
  onSignatureAdded,
  onSubmitTransaction,
}: SignatureCollectorProps) {
  const [signatures, setSignatures] = useState<Signature[]>([]);
  const [signerInput, setSignerInput] = useState('');
  const [signatureInput, setSignatureInput] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const thresholdMet = signatures.length >= requiredSignatures;

  const handleAddSignature = useCallback(async () => {
    if (!signerInput.trim() || !signatureInput.trim()) {
      setError('Both signer address and signature are required');
      return;
    }
    if (signatures.some((s) => s.signer === signerInput.trim())) {
      setError('This signer has already added a signature');
      return;
    }

    const sig: Signature = {
      signer: signerInput.trim(),
      signature: signatureInput.trim(),
    };

    setError(null);
    try {
      await onSignatureAdded(transactionId, sig);
      setSignatures((prev) => [...prev, sig]);
      setSignerInput('');
      setSignatureInput('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add signature');
    }
  }, [signerInput, signatureInput, signatures, onSignatureAdded, transactionId]);

  const handleSubmit = useCallback(async () => {
    if (!thresholdMet) return;
    setSubmitting(true);
    setError(null);
    setSuccess(null);
    try {
      await onSubmitTransaction(transactionId);
      setSuccess('Transaction submitted successfully');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to submit transaction');
    } finally {
      setSubmitting(false);
    }
  }, [thresholdMet, onSubmitTransaction, transactionId]);

  return (
    <div className="p-6 bg-slate-900/50 rounded-xl border border-slate-800 space-y-6">
      <div>
        <h2 className="text-xl font-bold mb-2">Signature Collection</h2>
        <p className="text-sm text-slate-400">
          Transaction ID: <code className="text-slate-300">{transactionId}</code>
        </p>
      </div>

      {/* Progress indicator */}
      <div className="flex items-center gap-3">
        <div className="flex-1">
          <div className="flex justify-between text-sm mb-1">
            <span className="text-slate-400">Signatures collected</span>
            <span className={thresholdMet ? 'text-green-400 font-semibold' : 'text-slate-300'}>
              {signatures.length} / {requiredSignatures}
            </span>
          </div>
          <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
            <div
              className={`h-full transition-all duration-300 ${thresholdMet ? 'bg-green-500' : 'bg-blue-500'}`}
              style={{ width: `${Math.min(100, (signatures.length / requiredSignatures) * 100)}%` }}
            />
          </div>
        </div>
      </div>

      {/* Collected signatures list */}
      {signatures.length > 0 && (
        <div className="space-y-2">
          <h3 className="text-sm font-semibold text-slate-300">Collected Signatures</h3>
          {signatures.map((sig, idx) => (
            <div key={idx} className="flex items-center gap-2 p-2 bg-slate-800/50 rounded-lg">
              <span className="text-green-400">✓</span>
              <span className="text-sm font-mono text-slate-300 truncate flex-1">
                {sig.signer.slice(0, 12)}...{sig.signer.slice(-6)}
              </span>
              <span className="text-xs text-slate-500">Signed</span>
            </div>
          ))}
        </div>
      )}

      {/* Add signature form (only if threshold not met) */}
      {!thresholdMet && (
        <div className="space-y-3">
          <h3 className="text-sm font-semibold text-slate-300">Add Signature</h3>
          <input
            type="text"
            placeholder="Signer address (G...)"
            value={signerInput}
            onChange={(e) => setSignerInput(e.target.value)}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-sm text-slate-200 placeholder-slate-500 focus:border-blue-500 focus:outline-none"
          />
          <textarea
            placeholder="Signature (base64)"
            value={signatureInput}
            onChange={(e) => setSignatureInput(e.target.value)}
            rows={3}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-sm text-slate-200 placeholder-slate-500 focus:border-blue-500 focus:outline-none font-mono"
          />
          <button
            onClick={handleAddSignature}
            disabled={!signerInput.trim() || !signatureInput.trim()}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-700 disabled:cursor-not-allowed text-white text-sm font-semibold rounded-lg transition-colors"
          >
            Add Signature
          </button>
        </div>
      )}

      {/* XDR preview */}
      {xdr && (
        <div>
          <h3 className="text-sm font-semibold text-slate-300 mb-1">Transaction XDR</h3>
          <div className="p-3 bg-slate-800/50 rounded-lg overflow-x-auto">
            <code className="text-xs text-slate-400 break-all">{xdr}</code>
          </div>
        </div>
      )}

      {/* Error/Success messages */}
      {error && (
        <div className="p-3 bg-red-900/30 border border-red-800 rounded-lg text-sm text-red-400">
          {error}
        </div>
      )}
      {success && (
        <div className="p-3 bg-green-900/30 border border-green-800 rounded-lg text-sm text-green-400">
          {success}
        </div>
      )}

      {/* Submit button */}
      <button
        onClick={handleSubmit}
        disabled={!thresholdMet || submitting}
        className="w-full px-4 py-3 bg-green-600 hover:bg-green-700 disabled:bg-slate-700 disabled:cursor-not-allowed text-white font-semibold rounded-lg transition-colors"
      >
        {submitting
          ? 'Submitting...'
          : thresholdMet
          ? 'Submit Transaction'
          : `Waiting for ${requiredSignatures - signatures.length} more signature(s)`}
      </button>
    </div>
  );
}
