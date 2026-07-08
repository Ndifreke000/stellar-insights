'use client';

import React from 'react';

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

export function SignatureCollector(_props: SignatureCollectorProps) {
    return (
        <div className="p-8 text-center bg-slate-900/50 rounded-xl border border-slate-800">
            <h2 className="text-xl font-bold mb-4">Signature Collection</h2>
            <p className="text-slate-400">
                This component is currently undergoing maintenance.
            </p>
        </div>
    );
}
