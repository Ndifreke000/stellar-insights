"use client";
import { logger } from "@/lib/logger";
import React, { useState } from "react";
import { TransactionBuilder } from "@/components/transactions/TransactionBuilder";
import {
  SignatureCollector,
  Signature,
} from "@/components/transactions/SignatureCollector";
import { Hexagon } from "lucide-react";
import { useWallet } from "@/components/lib/wallet-context";
import { config } from "@/config";

/** Backend API root, e.g. http://localhost:8080 (without a trailing /api). */
const API_ROOT = config.apiUrl.replace(/\/api\/?$/, "").replace(/\/$/, "");

/** Read the server's error message, falling back to the status text. */
async function errorMessage(res: Response): Promise<string> {
  const text = await res.text();
  return text || res.statusText;
}

export default function TransactionsBuilderPage() {
  // Every transactions endpoint needs the wallet's SEP-10 session, and the
  // transaction's source must be that wallet.
  const { address, authToken } = useWallet();
  const authHeaders = (): Record<string, string> =>
    authToken ? { Authorization: `Bearer ${authToken}` } : {};
  const [transactionId, setTransactionId] = useState<string | null>(null);
  const [xdr, setXdr] = useState<string | null>(null);
  const [requiredSignatures, setRequiredSignatures] = useState(1);
  const [_loading, setLoading] = useState(false);

  const handleXdrGenerated = async (
    generatedXdr: string,
    requiredSigs: number,
  ) => {
    try {
      if (!address || !authToken) {
        throw new Error("Connect and sign in with your wallet first.");
      }
      setLoading(true);
      const res = await fetch(`${API_ROOT}/api/transactions`, {
        method: "POST",
        headers: { "Content-Type": "application/json", ...authHeaders() },
        body: JSON.stringify({
          source_account: address,
          xdr: generatedXdr,
          required_signatures: requiredSigs,
        }),
      });

      if (!res.ok) {
        throw new Error(await errorMessage(res));
      }

      const data = await res.json();
      setTransactionId(data.id);
      setXdr(generatedXdr);
      setRequiredSignatures(requiredSigs);
    } catch (err) {
      logger.error(err as string);
      alert("Error creating transaction: " + err);
    } finally {
      setLoading(false);
    }
  };

  const handleSignatureAdded = async (txId: string, sig: Signature) => {
    const res = await fetch(`${API_ROOT}/api/transactions/${txId}/signatures`, {
      method: "POST",
      headers: { "Content-Type": "application/json", ...authHeaders() },
      body: JSON.stringify({
        signer: sig.signer,
        signature: sig.signature,
      }),
    });

    if (!res.ok) {
      throw new Error(await errorMessage(res));
    }
  };

  const handleSubmitTransaction = async (txId: string) => {
    const res = await fetch(`${API_ROOT}/api/transactions/${txId}/submit`, {
      method: "POST",
      headers: authHeaders(),
    });

    if (!res.ok) {
      throw new Error(await errorMessage(res));
    }
  };

  return (
    <div className="min-h-screen bg-slate-950 p-6 md:p-12 text-slate-200 font-sans">
      <div className="max-w-4xl mx-auto space-y-8">
        {/* Header Section */}
        <div className="flex flex-col mb-10">
          <div className="flex items-center gap-3 mb-2">
            <div className="p-2.5 rounded-xl bg-gradient-to-br from-indigo-500/20 to-purple-500/20 border border-indigo-500/20">
              <Hexagon className="w-6 h-6 text-indigo-400" />
            </div>
            <h1 className="text-3xl font-extrabold tracking-tight text-white">
              Multi-Sig Workflows
            </h1>
          </div>
          <p className="text-slate-400 text-sm md:text-base max-w-2xl">
            Build and authorize operations required for institutional anchors
            securely. Generate your transactions, collect signatures
            sequentially, and submit when perfectly aligned.
          </p>
        </div>

        {/* Builder / Collector Toggle */}
        <div className="flex flex-col items-center justify-center w-full mt-10">
          {!transactionId ? (
            <TransactionBuilder onXdrGenerated={handleXdrGenerated} />
          ) : (
            <div className="w-full flex justify-center animate-in fade-in slide-in-from-bottom-4 duration-700">
              <SignatureCollector
                transactionId={transactionId}
                xdr={xdr!}
                requiredSignatures={requiredSignatures}
                onSignatureAdded={handleSignatureAdded}
                onSubmitTransaction={handleSubmitTransaction}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
