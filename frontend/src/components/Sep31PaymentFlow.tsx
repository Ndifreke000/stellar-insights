"use client";

import React, { useCallback, useState } from "react";
import { useForm, SubmitHandler } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import {
  getSep31Anchors,
  getSep31Info,
  getSep31Quote,
  createSep31Payment,
  getSep31Transactions,
  type Sep31AnchorInfo,
  type Sep31InfoResponse,
  type Sep31Transaction,
  type Sep31QuoteResponse,
  Sep31Error,
} from "@/services/sep31";
import {
  Send,
  Loader2,
  RefreshCw,
  AlertCircle,
  CheckCircle,
  Clock,
  Banknote,
  Quote,
} from "lucide-react";
import { FormField, FormSelect } from "@/components/ui/FormField";
import { sep31PaymentFlowSchema, type Sep31PaymentFlowForm } from "@/lib/schemas";

interface ComplianceField {
  name: string;
  description?: string;
  optional?: boolean;
}

export function Sep31PaymentFlow() {
  const [anchors, setAnchors] = useState<Sep31AnchorInfo[]>([]);
  const [selectedAnchor, setSelectedAnchor] = useState<Sep31AnchorInfo | null>(null);
  const [info, setInfo] = useState<Sep31InfoResponse | null>(null);
  const [complianceFields, setComplianceFields] = useState<ComplianceField[]>([]);
  const [complianceValues, setComplianceValues] = useState<Record<string, string>>({});
  const [complianceErrors, setComplianceErrors] = useState<Record<string, string>>({});
  const [transactions, setTransactions] = useState<Sep31Transaction[]>([]);
  const [quote, setQuote] = useState<Sep31QuoteResponse | null>(null);
  const [loadingAnchors, setLoadingAnchors] = useState(false);
  const [loadingInfo, setLoadingInfo] = useState(false);
  const [loadingQuote, setLoadingQuote] = useState(false);
  const [loadingTx, setLoadingTx] = useState(false);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    formState: { errors, isValid, isDirty },
    setValue,
    watch,
    trigger,
    resetField,
  } = useForm<Sep31PaymentFlowForm>({
    resolver: zodResolver(sep31PaymentFlowSchema),
    mode: "onChange",
    defaultValues: {
      transferServer: "",
      amount: "",
      receiverId: "",
      sourceAsset: "",
      destAsset: "",
      jwt: "",
    },
  });

  // Watch form values for real-time updates
  const transferServer = watch("transferServer");
  const amount = watch("amount");
  const receiverId = watch("receiverId");
  const sourceAsset = watch("sourceAsset");
  const destAsset = watch("destAsset");
  const jwt = watch("jwt");

  // Update selected anchor when transfer server changes
  React.useEffect(() => {
    if (transferServer) {
      const anchor = anchors.find((a) => a.transfer_server === transferServer);
      setSelectedAnchor(anchor || null);
    } else {
      setSelectedAnchor(null);
    }
  }, [transferServer, anchors]);

  const isFormValid = isValid && isDirty;

  const resolvedTransferServer = selectedAnchor?.transfer_server || transferServer?.trim();

  const loadAnchors = useCallback(async () => {
    setLoadingAnchors(true);
    setError(null);
    try {
      const res = await getSep31Anchors();
      setAnchors(res.anchors || []);
      if (res.anchors?.length && !selectedAnchor) {
        setSelectedAnchor(res.anchors[0]);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load anchors");
    } finally {
      setLoadingAnchors(false);
    }
  }, [selectedAnchor]);

  const loadInfo = useCallback(async () => {
    if (!resolvedTransferServer) {
      setInfo(null);
      setComplianceFields([]);
      return;
    }
    setLoadingInfo(true);
    setError(null);
    try {
      const data = await getSep31Info(resolvedTransferServer);
      setInfo(data);

      const fields: ComplianceField[] = [];
      if (data.receive) {
        for (const asset of Object.values(data.receive)) {
          const types = (asset as Record<string, unknown>).types as
            | Record<string, { fields?: Record<string, { description?: string; optional?: boolean }> }>
            | undefined;
          if (!types) continue;
          for (const typeInfo of Object.values(types)) {
            if (!typeInfo.fields) continue;
            for (const [fieldName, fieldMeta] of Object.entries(typeInfo.fields)) {
              if (!fields.some((f) => f.name === fieldName)) {
                fields.push({
                  name: fieldName,
                  description: fieldMeta.description,
                  optional: fieldMeta.optional,
                });
              }
            }
          }
        }
      }
      setComplianceFields(fields);
      setComplianceValues({});
      setComplianceErrors({});
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load anchor info");
      setInfo(null);
      setComplianceFields([]);
    } finally {
      setLoadingInfo(false);
    }
  }, [resolvedTransferServer]);

  const loadQuote = useCallback(async () => {
    if (!resolvedTransferServer || !amount) {
      setError("Enter amount and select an anchor");
      return;
    }
    setLoadingQuote(true);
    setError(null);
    setQuote(null);
    try {
      const res = await getSep31Quote({
        transfer_server: resolvedTransferServer,
        jwt: jwt || undefined,
        amount,
        sell_asset: sourceAsset || undefined,
        buy_asset: destAsset || undefined,
      });
      setQuote(res);
    } catch (e) {
      const msg =
        e instanceof Sep31Error
          ? e.message
          : e instanceof Error
            ? e.message
            : "Failed to get quote";
      setError(msg);
    } finally {
      setLoadingQuote(false);
    }
  }, [resolvedTransferServer, amount, sourceAsset, destAsset, jwt]);

  const loadTransactions = useCallback(async () => {
    if (!resolvedTransferServer) return;
    setLoadingTx(true);
    setError(null);
    try {
      const res = await getSep31Transactions({
        transfer_server: resolvedTransferServer,
        jwt: jwt || undefined,
        limit: 20,
      });
      setTransactions(res.transactions || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load payments");
      setTransactions([]);
    } finally {
      setLoadingTx(false);
    }
  }, [resolvedTransferServer, jwt]);

  React.useEffect(() => {
    loadAnchors();
  }, []);

  React.useEffect(() => {
    if (resolvedTransferServer) loadInfo();
    else setInfo(null);
  }, [resolvedTransferServer]);

  const validateComplianceFields = (): boolean => {
    const errors: Record<string, string> = {};
    for (const field of complianceFields) {
      if (!field.optional && !complianceValues[field.name]?.trim()) {
        errors[field.name] = `${field.name} is required`;
      }
    }
    setComplianceErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleSendPayment: SubmitHandler<Sep31PaymentFlowForm> = async (data) => {
    const base = selectedAnchor?.transfer_server || data.transferServer;
    if (!base) {
      setError("Select an anchor or enter transfer server URL");
      return;
    }
    if (!isFormValid) return;
    if (!validateComplianceFields()) {
      setError("Please fill in all required compliance fields");
      return;
    }
    setSending(true);
    setError(null);
    setSuccessMessage(null);
    try {
      const compliancePayload: Record<string, string> = {};
      for (const field of complianceFields) {
        const val = complianceValues[field.name]?.trim();
        if (val) compliancePayload[field.name] = val;
      }
      const res = await createSep31Payment({
        transfer_server: base,
        jwt: data.jwt || undefined,
        amount: data.amount,
        receiver_id: data.receiverId || undefined,
        quote_id: quote?.id || undefined,
        source_asset: data.sourceAsset || undefined,
        destination_asset: data.destAsset || undefined,
        ...compliancePayload,
      });
      const id = (res as { id?: string }).id ?? (res as { transaction?: Sep31Transaction }).transaction?.id;
      setSuccessMessage(
        id
          ? `Payment initiated. Transaction ID: ${id}. Complete KYC or sender/receiver flows on the anchor if required.`
          : "Payment submitted. Check payment history for status."
      );
      setQuote(null);
      setComplianceValues({});
      loadTransactions();
    } catch (e) {
      const msg =
        e instanceof Sep31Error
          ? e.message
          : e instanceof Error
            ? e.message
            : "Failed to send payment";
      setError(msg);
    } finally {
      setSending(false);
    }
  };

  const statusColor = (status: string) => {
    const s = status?.toLowerCase() || "";
    if (s.includes("complete") || s.includes("success")) return "text-emerald-400";
    if (s.includes("pending") || s.includes("processing")) return "text-amber-400";
    return "text-muted-foreground";
  };

  return (
    <div className="space-y-8">
      {/* Anchor selection */}
      <section className="glass-card rounded-2xl p-6">
        <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
          <Banknote className="w-5 h-5 text-accent" />
          Anchor & transfer server
        </h2>
        <div className="flex flex-wrap gap-4 items-end">
          <div className="flex-1 min-w-[200px]">
            <label className="block text-sm font-medium text-muted-foreground mb-1">
              Preset anchors
            </label>
            <select
              className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground focus:ring-2 focus:ring-accent/50"
              value={selectedAnchor?.transfer_server ?? ""}
              onChange={(e: React.ChangeEvent<HTMLSelectElement>) => {
                const a = anchors.find(
                  (x: Sep31AnchorInfo) => x.transfer_server === e.target.value
                );
                setSelectedAnchor(a || null);
                setValue("transferServer", e.target.value);
                trigger("transferServer");
              }}
              disabled={loadingAnchors}
            >
              <option value="">Select an anchor</option>
              {anchors.map((a) => (
                <option key={a.transfer_server} value={a.transfer_server}>
                  {a.name} ({a.transfer_server})
                </option>
              ))}
            </select>
          </div>
          <button
            type="button"
            onClick={loadAnchors}
            disabled={loadingAnchors}
            className="rounded-xl border border-border px-4 py-2.5 text-sm font-medium text-foreground hover:bg-white/5 flex items-center gap-2"
          >
            {loadingAnchors ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <RefreshCw className="w-4 h-4" />
            )}
            Refresh
          </button>
        </div>
        <div className="mt-4">
          <FormField
            name="transferServer"
            label="Or enter transfer server URL (SEP-31 base)"
            type="url"
            placeholder="https://api.anchor.example/sep31"
            description="Custom SEP-31 transfer server endpoint"
          />
        </div>
      </section>

      {/* Quote & Send payment */}
      <section className="glass-card rounded-2xl p-6">
        <h2 className="text-lg font-semibold text-foreground mb-4">
          Get quote & send payment
        </h2>
        <p className="text-muted-foreground text-sm mb-4">
          KYC may be required by the anchor for sender or receiver. Complete any interactive flows the anchor provides.
        </p>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
          <FormField
            name="amount"
            label="Amount"
            type="text"
            placeholder="100"
            required
          />
          <FormField
            name="receiverId"
            label="Receiver ID (anchor-specific)"
            type="text"
            placeholder="receiver@anchor.com or wallet id"
            description="Destination identifier as defined by the anchor"
            required
          />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
          <FormField
            name="sourceAsset"
            label="Source asset (e.g. USDC:issuer)"
            type="text"
            placeholder="optional"
            description="Asset you want to send (optional)"
          />
          <FormField
            name="destAsset"
            label="Destination asset (e.g. iso4217:USD)"
            type="text"
            placeholder="optional"
            description="Asset you want to receive (optional)"
          />
        </div>
        <div className="mb-4">
          <FormField
            name="jwt"
            label="JWT (optional, from SEP-10)"
            type="password"
            placeholder="For authenticated flows"
            description="Authentication token from SEP-10 challenge"
          />
        </div>
        {complianceFields.length > 0 && (
          <div className="mb-4 p-4 rounded-xl bg-white/5 border border-border">
            <h3 className="text-sm font-medium text-foreground mb-3">
              Anchor compliance fields
            </h3>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              {complianceFields.map((field) => (
                <div key={field.name}>
                  <label className="block text-sm font-medium text-muted-foreground mb-1">
                    {field.name}
                    {!field.optional && <span className="text-red-400 ml-1">*</span>}
                  </label>
                  <input
                    type="text"
                    value={complianceValues[field.name] || ""}
                    onChange={(e) => {
                      setComplianceValues((prev) => ({
                        ...prev,
                        [field.name]: e.target.value,
                      }));
                      if (complianceErrors[field.name]) {
                        setComplianceErrors((prev) => {
                          const next = { ...prev };
                          delete next[field.name];
                          return next;
                        });
                      }
                    }}
                    placeholder={field.description || field.name}
                    className="w-full rounded-xl bg-background/80 border border-border px-4 py-2.5 text-foreground focus:ring-2 focus:ring-accent/50"
                  />
                  {complianceErrors[field.name] && (
                    <p className="text-red-400 text-xs mt-1">
                      {complianceErrors[field.name]}
                    </p>
                  )}
                  {field.description && (
                    <p className="text-muted-foreground text-xs mt-1">
                      {field.description}
                    </p>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
        {error && (
          <div className="mb-4 flex items-center gap-2 rounded-xl bg-red-500/10 border border-red-500/20 px-4 py-3 text-red-400 text-sm">
            <AlertCircle className="w-4 h-4 shrink-0" />
            {error}
          </div>
        )}
        {successMessage && (
          <div className="mb-4 flex items-center gap-2 rounded-xl bg-emerald-500/10 border border-emerald-500/20 px-4 py-3 text-emerald-400 text-sm">
            <CheckCircle className="w-4 h-4 shrink-0" />
            {successMessage}
          </div>
        )}
        <div className="flex flex-wrap gap-3">
          <button
            type="button"
            onClick={loadQuote}
            disabled={loadingQuote || !resolvedTransferServer || !amount || !isFormValid}
            className="rounded-xl border border-border px-4 py-2.5 text-sm font-medium text-foreground hover:bg-white/5 flex items-center gap-2 disabled:opacity-50"
          >
            {loadingQuote ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Quote className="w-4 h-4" />
            )}
            Get quote
          </button>
          <button
            type="button"
            onClick={handleSubmit(handleSendPayment)}
            disabled={sending || !resolvedTransferServer || !amount || !isFormValid}
            className="rounded-xl bg-accent text-accent-foreground px-6 py-2.5 font-medium hover:opacity-90 flex items-center gap-2 disabled:opacity-50"
          >
            {sending ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Send className="w-4 h-4" />
            )}
            Send payment
          </button>
        </div>
        {quote && (
          <div className="mt-4 p-4 rounded-xl bg-white/5 border border-border">
            <div className="text-sm font-medium text-muted-foreground mb-2 flex items-center gap-2">
              <Quote className="w-4 h-4" />
              Quote
            </div>
            <pre className="text-xs text-foreground overflow-x-auto">
              {JSON.stringify(quote, null, 2)}
            </pre>
            {quote.id && (
              <p className="text-xs text-muted-foreground mt-2">
                Quote ID: {quote.id}. Use this when sending the payment if the anchor requires it.
              </p>
            )}
          </div>
        )}
        {resolvedTransferServer && !info && loadingInfo && (
          <div className="flex items-center gap-2 text-muted-foreground py-4">
            <Loader2 className="w-4 h-4 animate-spin" />
            Loading anchor capabilities…
          </div>
        )}
      </section>

      {/* Payment history */}
      <section className="glass-card rounded-2xl p-6">
        <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
          <Clock className="w-5 h-5 text-accent" />
          Payment history
        </h2>
        {resolvedTransferServer && (
          <button
            type=”button”
            onClick={loadTransactions}
            disabled={loadingTx}
            className=”mb-4 rounded-xl border border-border px-4 py-2 text-sm font-medium text-foreground hover:bg-white/5 flex items-center gap-2”
          >
            {loadingTx ? (
              <Loader2 className=”w-4 h-4 animate-spin” />
            ) : (
              <RefreshCw className=”w-4 h-4” />
            )}
            Load history
          </button>
        )}
        {transactions.length === 0 && !loadingTx && (
          <p className=”text-muted-foreground text-sm”>
            {resolvedTransferServer
              ? “Click “Load history” to fetch payments (JWT may be required).”
              : “Select an anchor above to load payment history.”}
          </p>
        )}
        {transactions.length > 0 && (
          <div className="overflow-x-auto rounded-xl border border-border">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-white/5">
                  <th className="text-left py-3 px-4 font-medium text-muted-foreground">
                    ID
                  </th>
                  <th className="text-left py-3 px-4 font-medium text-muted-foreground">
                    Amount
                  </th>
                  <th className="text-left py-3 px-4 font-medium text-muted-foreground">
                    Status
                  </th>
                  <th className="text-left py-3 px-4 font-medium text-muted-foreground">
                    Date
                  </th>
                </tr>
              </thead>
              <tbody>
                {transactions.map((tx: Sep31Transaction) => (
                  <tr
                    key={tx.id}
                    className="border-b border-border/50 hover:bg-white/5"
                  >
                    <td className="py-3 px-4 font-mono text-foreground text-xs">
                      {typeof tx.id === "string"
                        ? tx.id.length > 12
                          ? `${tx.id.slice(0, 12)}…`
                          : tx.id
                        : tx.id ?? "—"}
                    </td>
                    <td className="py-3 px-4 text-foreground">
                      {tx.amount ?? tx.amount_out ?? "—"}
                    </td>
                    <td className={`py-3 px-4 ${statusColor(tx.status)}`}>
                      {tx.status}
                    </td>
                    <td className="py-3 px-4 text-muted-foreground">
                      {tx.created_at
                        ? new Date(tx.created_at).toLocaleString()
                        : "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}
