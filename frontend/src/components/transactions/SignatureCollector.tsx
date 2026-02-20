import React, { useState } from 'react';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { KeyRound, PenLine, Send, CheckCircle2, Copy } from 'lucide-react';
import * as StellarSdk from '@stellar/stellar-sdk';

export interface Signature {
    signer: string;
    signature: string;
}

export interface SignatureCollectorProps {
    transactionId: string;
    xdr: string;
    requiredSignatures: number;
    initialSignatures?: Signature[];
    onSignatureAdded: (txId: string, sig: Signature) => Promise<void>;
    onSubmitTransaction: (txId: string) => Promise<void>;
}

export function SignatureCollector({
    transactionId,
    xdr,
    requiredSignatures,
    initialSignatures = [],
    onSignatureAdded,
    onSubmitTransaction,
}: SignatureCollectorProps) {
    const [signatures, setSignatures] = useState<Signature[]>(initialSignatures);
    const [newSigner, setNewSigner] = useState('');
    const [newSigValue, setNewSigValue] = useState('');
    const [secretKey, setSecretKey] = useState('');
    const [isSignMode, setIsSignMode] = useState(false);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState('');
    const [submitted, setSubmitted] = useState(false);

    const handleAddSignature = async () => {
        try {
            setLoading(true);
            setError('');

            let sigToAdd: Signature;

            if (isSignMode) {
                if (!secretKey) throw new Error('Secret key is required to sign');
                const keypair = StellarSdk.Keypair.fromSecret(secretKey);
                const tx = StellarSdk.TransactionBuilder.fromXDR(xdr, StellarSdk.Networks.TESTNET);
                tx.sign(keypair);

                // Extract the newly added signature 
                const addedSig = tx.signatures[tx.signatures.length - 1];
                sigToAdd = {
                    signer: keypair.publicKey(),
                    signature: addedSig.signature().toString('base64'),
                };
            } else {
                if (!newSigner || !newSigValue) throw new Error('Signer public key and signature base64 are required');
                sigToAdd = {
                    signer: newSigner,
                    signature: newSigValue,
                };
            }

            await onSignatureAdded(transactionId, sigToAdd);
            setSignatures([...signatures, sigToAdd]);

            setNewSigner('');
            setNewSigValue('');
            setSecretKey('');
        } catch (err: unknown) {
            setError(err instanceof Error ? err.message : 'Failed to add signature');
        } finally {
            setLoading(false);
        }
    };

    const handleSubmit = async () => {
        try {
            setLoading(true);
            setError('');
            await onSubmitTransaction(transactionId);
            setSubmitted(true);
        } catch (err: unknown) {
            setError(err instanceof Error ? err.message : 'Failed to submit transaction');
        } finally {
            setLoading(false);
        }
    };

    const copyXdr = () => {
        navigator.clipboard.writeText(xdr);
    };

    const progress = Math.min(100, (signatures.length / requiredSignatures) * 100);
    const canSubmit = signatures.length >= requiredSignatures;

    if (submitted) {
        return (
            <Card className="w-full max-w-2xl border-emerald-900/50 bg-slate-900/50 backdrop-blur-xl">
                <CardContent className="pt-10 pb-10 flex flex-col items-center justify-center text-center space-y-4">
                    <div className="w-16 h-16 rounded-full bg-emerald-500/20 flex items-center justify-center mb-4">
                        <CheckCircle2 className="w-8 h-8 text-emerald-400" />
                    </div>
                    <h2 className="text-2xl font-bold text-white">Transaction Submitted</h2>
                    <p className="text-slate-400">
                        The multi-signature transaction has been fully collected and submitted to the network.
                    </p>
                </CardContent>
            </Card>
        );
    }

    return (
        <Card className="w-full max-w-2xl border-slate-800 bg-slate-900/50 backdrop-blur-xl">
            <CardHeader>
                <div className="flex items-center gap-3">
                    <div className="p-2 bg-indigo-500/10 rounded-lg">
                        <KeyRound className="w-6 h-6 text-indigo-400" />
                    </div>
                    <div className="flex-1">
                        <CardTitle className="text-xl font-bold bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-transparent">
                            Signature Collection
                        </CardTitle>
                        <CardDescription className="text-slate-400">
                            Collect {requiredSignatures} signatures to authorize this transaction
                        </CardDescription>
                    </div>
                </div>
            </CardHeader>

            <CardContent className="space-y-6">
                <div className="space-y-2 relative">
                    <div className="flex items-center justify-between">
                        <Label className="text-slate-300">Transaction XDR</Label>
                        <Button variant="ghost" size="sm" onClick={copyXdr} className="h-6 px-2 text-xs text-slate-400 hover:text-white">
                            <Copy className="w-3 h-3 mr-1" /> Copy
                        </Button>
                    </div>
                    <div className="p-3 font-mono text-xs text-slate-400 bg-slate-950 rounded-md break-all border border-slate-800 max-h-32 overflow-y-auto">
                        {xdr}
                    </div>
                </div>

                <div className="space-y-4 pt-4 border-t border-slate-800">
                    <div className="flex items-center justify-between mb-2">
                        <Label className="text-slate-300">Progress: {signatures.length} / {requiredSignatures} Signatures</Label>
                        <span className="text-xs text-slate-400 font-medium">{Math.floor(progress)}%</span>
                    </div>
                    <div className="h-2 w-full bg-slate-800 rounded-full overflow-hidden">
                        <div
                            className="h-full bg-gradient-to-r from-indigo-500 to-purple-500 transition-all duration-500 ease-out"
                            style={{ width: `${progress}%` }}
                        />
                    </div>

                    <div className="space-y-2 mt-4">
                        {signatures.map((sig, i) => (
                            <div key={i} className="flex items-center justify-between p-3 rounded-md bg-slate-800/50 border border-slate-700/50">
                                <div className="flex items-center gap-2">
                                    <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                                    <span className="text-sm font-medium text-slate-300">
                                        {sig.signer.substring(0, 6)}...{sig.signer.substring(sig.signer.length - 4)}
                                    </span>
                                </div>
                                <span className="text-xs text-slate-500 font-mono">Signed</span>
                            </div>
                        ))}
                    </div>
                </div>

                <div className="pt-4 border-t border-slate-800 space-y-4">
                    <div className="flex items-center gap-4 mb-4">
                        <button
                            className={`text-sm font-medium pb-2 border-b-2 ${!isSignMode ? 'border-indigo-400 text-indigo-400' : 'border-transparent text-slate-400 hover:text-slate-300'}`}
                            onClick={() => setIsSignMode(false)}
                        >
                            Provide Signature
                        </button>
                        <button
                            className={`text-sm font-medium pb-2 border-b-2 ${isSignMode ? 'border-indigo-400 text-indigo-400' : 'border-transparent text-slate-400 hover:text-slate-300'}`}
                            onClick={() => setIsSignMode(true)}
                        >
                            Sign Now (Secret Key)
                        </button>
                    </div>

                    {isSignMode ? (
                        <div className="space-y-3">
                            <Label className="text-xs text-slate-400">Signer Secret Key (S...)</Label>
                            <Input
                                type="password"
                                value={secretKey}
                                onChange={(e) => setSecretKey(e.target.value)}
                                placeholder="S..."
                                className="bg-slate-900 border-slate-700 font-mono text-sm"
                            />
                        </div>
                    ) : (
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div className="space-y-2">
                                <Label className="text-xs text-slate-400">Signer Public Key (G...)</Label>
                                <Input
                                    value={newSigner}
                                    onChange={(e) => setNewSigner(e.target.value)}
                                    placeholder="G..."
                                    className="bg-slate-900 border-slate-700 font-mono text-sm"
                                />
                            </div>
                            <div className="space-y-2">
                                <Label className="text-xs text-slate-400">Signature (Base64)</Label>
                                <Input
                                    value={newSigValue}
                                    onChange={(e) => setNewSigValue(e.target.value)}
                                    placeholder="..."
                                    className="bg-slate-900 border-slate-700 font-mono text-sm"
                                />
                            </div>
                        </div>
                    )}

                    <Button
                        onClick={handleAddSignature}
                        disabled={loading}
                        variant="secondary"
                        className="w-full bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700"
                    >
                        <PenLine className="w-4 h-4 mr-2" />
                        Add Signature
                    </Button>

                    {error && <div className="p-3 text-sm text-red-400 bg-red-400/10 rounded-md border border-red-400/20">{error}</div>}
                </div>

            </CardContent>
            <CardFooter className="bg-slate-900/80 border-t border-slate-800 p-6">
                <Button
                    onClick={handleSubmit}
                    disabled={!canSubmit || loading}
                    className={`w-full font-semibold shadow-lg ${canSubmit
                        ? 'bg-gradient-to-r from-indigo-500 to-purple-500 hover:from-indigo-400 hover:to-purple-400 text-white shadow-indigo-500/25'
                        : 'bg-slate-800 text-slate-500 cursor-not-allowed border-none'
                        }`}
                >
                    <Send className="w-4 h-4 mr-2" />
                    {canSubmit ? 'Submit Transaction' : `Waiting for ${requiredSignatures - signatures.length} more signature(s)`}
                </Button>
            </CardFooter>
        </Card>
    );
}
