import React, { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { PlusCircle, Trash2, ShieldCheck, Cpu } from 'lucide-react';
import * as StellarSdk from '@stellar/stellar-sdk';

export interface Operation {
  id: string;
  type: 'payment' | 'changeTrust' | 'manageData';
  params: Record<string, string>;
}

export interface Signer {
  publicKey: string;
}

export interface TransactionBuilderProps {
  onXdrGenerated: (xdr: string, requiredSignatures: number) => void;
}

export function TransactionBuilder({ onXdrGenerated }: TransactionBuilderProps) {
  const [sourceAccount, setSourceAccount] = useState('');
  const [sequenceNumber, setSequenceNumber] = useState('');
  const [requiredSignatures, setRequiredSignatures] = useState(1);
  const [operations, setOperations] = useState<Operation[]>([]);
  const [error, setError] = useState('');

  const addOperation = () => {
    setOperations([
      ...operations,
      { id: Math.random().toString(36).substring(7), type: 'payment', params: {} }
    ]);
  };

  const removeOperation = (id: string) => {
    setOperations(operations.filter((op) => op.id !== id));
  };

  const updateOperation = (id: string, type: Operation['type']) => {
    setOperations(operations.map((op) => (op.id === id ? { ...op, type, params: {} } : op)));
  };

  const updateOperationParam = (id: string, key: string, value: string) => {
    setOperations(
      operations.map((op) =>
        op.id === id ? { ...op, params: { ...op.params, [key]: value } } : op
      )
    );
  };

  const generateXdr = async () => {
    try {
      setError('');
      if (!sourceAccount) throw new Error('Source account is required');
      if (!sequenceNumber) throw new Error('Sequence number is required');
      if (operations.length === 0) throw new Error('At least one operation is required');

      // Create dummy account for building tx without network fetch
      const account = new StellarSdk.Account(sourceAccount, sequenceNumber);
      let builder = new StellarSdk.TransactionBuilder(account, {
        fee: StellarSdk.BASE_FEE,
        networkPassphrase: StellarSdk.Networks.TESTNET,
      });

      operations.forEach((op) => {
        if (op.type === 'payment') {
          if (!op.params.destination || !op.params.amount) throw new Error('Payment requires destination and amount');
          builder = builder.addOperation(
            StellarSdk.Operation.payment({
              destination: op.params.destination,
              asset: StellarSdk.Asset.native(),
              amount: op.params.amount,
            })
          );
        } else if (op.type === 'changeTrust') {
          if (!op.params.assetCode || !op.params.assetIssuer) throw new Error('ChangeTrust requires asset details');
          builder = builder.addOperation(
            StellarSdk.Operation.changeTrust({
              asset: new StellarSdk.Asset(op.params.assetCode, op.params.assetIssuer),
            })
          );
        } else if (op.type === 'manageData') {
          if (!op.params.name || !op.params.value) throw new Error('ManageData requires name and value');
          builder = builder.addOperation(
            StellarSdk.Operation.manageData({
              name: op.params.name,
              value: op.params.value,
            })
          );
        }
      });

      const tx = builder.setTimeout(StellarSdk.TimeoutInfinite).build();
      const xdr = tx.toXDR();
      onXdrGenerated(xdr, requiredSignatures);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to generate XDR');
    }
  };

  return (
    <Card className="w-full max-w-3xl border-slate-800 bg-slate-900/50 backdrop-blur-xl">
      <CardHeader>
        <div className="flex items-center gap-3">
          <div className="p-2 bg-blue-500/10 rounded-lg">
            <Cpu className="w-6 h-6 text-blue-400" />
          </div>
          <div>
            <CardTitle className="text-xl font-bold bg-gradient-to-r from-blue-400 to-indigo-400 bg-clip-text text-transparent">
              Transaction Builder
            </CardTitle>
            <CardDescription className="text-slate-400">
              Construct complex operations for multi-signature authorization
            </CardDescription>
          </div>
        </div>
      </CardHeader>

      <CardContent className="space-y-6">
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label className="text-slate-300">Source Account</Label>
            <Input
              value={sourceAccount}
              onChange={(e) => setSourceAccount(e.target.value)}
              placeholder="G..."
              className="bg-slate-900 border-slate-700"
            />
          </div>
          <div className="space-y-2">
            <Label className="text-slate-300">Sequence Number</Label>
            <Input
              value={sequenceNumber}
              onChange={(e) => setSequenceNumber(e.target.value)}
              placeholder="12345678"
              className="bg-slate-900 border-slate-700"
            />
          </div>
        </div>

        <div className="space-y-2">
          <Label className="text-slate-300">Required Signatures Threshold</Label>
          <Input
            type="number"
            min={1}
            value={requiredSignatures}
            onChange={(e) => setRequiredSignatures(parseInt(e.target.value) || 1)}
            className="bg-slate-900 border-slate-700 w-1/3"
          />
        </div>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <Label className="text-slate-300 text-lg font-semibold">Operations</Label>
            <Button onClick={addOperation} variant="outline" size="sm" className="border-blue-500/30 text-blue-400 hover:bg-blue-500/10">
              <PlusCircle className="w-4 h-4 mr-2" />
              Add Operation
            </Button>
          </div>

          {operations.length === 0 ? (
            <div className="p-8 text-center border border-dashed border-slate-700 rounded-lg text-slate-500">
              No operations added. Click &quot;Add Operation&quot; to begin.
            </div>
          ) : (
            <div className="space-y-3">
              {operations.map((op, index) => (
                <div key={op.id} className="p-4 rounded-lg bg-slate-900/80 border border-slate-800 space-y-4">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium text-slate-400">Operation {index + 1}</span>
                    <Button variant="ghost" size="icon" onClick={() => removeOperation(op.id)} className="h-8 w-8 text-red-400 hover:bg-red-400/10 hover:text-red-300">
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div className="space-y-2">
                      <Label className="text-xs text-slate-400">Type</Label>
                      <Select value={op.type} onValueChange={(val: Operation['type']) => updateOperation(op.id, val)}>
                        <SelectTrigger className="bg-slate-800 border-slate-700">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="payment">Payment</SelectItem>
                          <SelectItem value="changeTrust">Change Trust</SelectItem>
                          <SelectItem value="manageData">Manage Data</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>

                    {op.type === 'payment' && (
                      <>
                        <div className="space-y-2">
                          <Label className="text-xs text-slate-400">Destination</Label>
                          <Input
                            value={op.params.destination || ''}
                            onChange={(e) => updateOperationParam(op.id, 'destination', e.target.value)}
                            placeholder="G..."
                            className="bg-slate-800 border-slate-700 h-9"
                          />
                        </div>
                        <div className="space-y-2">
                          <Label className="text-xs text-slate-400">Amount (XLM)</Label>
                          <Input
                            value={op.params.amount || ''}
                            onChange={(e) => updateOperationParam(op.id, 'amount', e.target.value)}
                            placeholder="100.00"
                            className="bg-slate-800 border-slate-700 h-9"
                          />
                        </div>
                      </>
                    )}

                    {op.type === 'changeTrust' && (
                      <>
                        <div className="space-y-2">
                          <Label className="text-xs text-slate-400">Asset Code</Label>
                          <Input
                            value={op.params.assetCode || ''}
                            onChange={(e) => updateOperationParam(op.id, 'assetCode', e.target.value)}
                            placeholder="USDC"
                            className="bg-slate-800 border-slate-700 h-9"
                          />
                        </div>
                        <div className="space-y-2">
                          <Label className="text-xs text-slate-400">Asset Issuer</Label>
                          <Input
                            value={op.params.assetIssuer || ''}
                            onChange={(e) => updateOperationParam(op.id, 'assetIssuer', e.target.value)}
                            placeholder="G..."
                            className="bg-slate-800 border-slate-700 h-9"
                          />
                        </div>
                      </>
                    )}

                    {op.type === 'manageData' && (
                      <>
                        <div className="space-y-2">
                          <Label className="text-xs text-slate-400">Name</Label>
                          <Input
                            value={op.params.name || ''}
                            onChange={(e) => updateOperationParam(op.id, 'name', e.target.value)}
                            placeholder="Key"
                            className="bg-slate-800 border-slate-700 h-9"
                          />
                        </div>
                        <div className="space-y-2">
                          <Label className="text-xs text-slate-400">Value (String)</Label>
                          <Input
                            value={op.params.value || ''}
                            onChange={(e) => updateOperationParam(op.id, 'value', e.target.value)}
                            placeholder="Data"
                            className="bg-slate-800 border-slate-700 h-9"
                          />
                        </div>
                      </>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {error && <div className="p-3 text-sm text-red-400 bg-red-400/10 rounded-md border border-red-400/20">{error}</div>}
      </CardContent>
      <CardFooter className="bg-slate-900/80 border-t border-slate-800 p-6">
        <Button onClick={generateXdr} className="w-full bg-blue-600 hover:bg-blue-500 text-white font-semibold shadow-lg shadow-blue-500/20">
          <ShieldCheck className="w-4 h-4 mr-2" />
          Generate Transaction XDR
        </Button>
      </CardFooter>
    </Card>
  );
}
