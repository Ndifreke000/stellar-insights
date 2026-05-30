"use client";

import React, { useEffect, useState, useCallback } from "react";
import {
  Key,
  Plus,
  Copy,
  Check,
  RefreshCw,
  Trash2,
  Eye,
  EyeOff,
  AlertTriangle,
  X,
  Shield,
} from "lucide-react";
import { useWallet } from "@/components/lib/wallet-context";
import {
  listApiKeys,
  createApiKey,
  rotateApiKey,
  revokeApiKey,
  type ApiKeyInfo,
  type CreateApiKeyResponse,
} from "@/lib/api-keys";

type ModalState =
  | { type: "none" }
  | { type: "create" }
  | { type: "reveal"; response: CreateApiKeyResponse }
  | { type: "rotate"; key: ApiKeyInfo }
  | { type: "revoke"; key: ApiKeyInfo };

export default function DeveloperKeysPage() {
  const { isConnected, address } = useWallet();
  const [keys, setKeys] = useState<ApiKeyInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalState>({ type: "none" });
  const [actionLoading, setActionLoading] = useState(false);

  const fetchKeys = useCallback(async () => {
    if (!address) return;
    setLoading(true);
    setError(null);
    try {
      const response = await listApiKeys(address);
      setKeys(response.keys);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load API keys");
    } finally {
      setLoading(false);
    }
  }, [address]);

  useEffect(() => {
    if (isConnected && address) {
      fetchKeys();
    } else {
      setLoading(false);
    }
  }, [isConnected, address, fetchKeys]);

  const handleCreate = async (
    name: string,
    scopes: string,
    expiresAt?: string,
  ) => {
    if (!address) return;
    setActionLoading(true);
    try {
      const response = await createApiKey(address, name, scopes, expiresAt);
      setModal({ type: "reveal", response });
      await fetchKeys();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to create API key",
      );
    } finally {
      setActionLoading(false);
    }
  };

  const handleRotate = async (id: string) => {
    if (!address) return;
    setActionLoading(true);
    try {
      const response = await rotateApiKey(address, id);
      setModal({ type: "reveal", response });
      await fetchKeys();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to rotate API key",
      );
    } finally {
      setActionLoading(false);
    }
  };

  const handleRevoke = async (id: string) => {
    if (!address) return;
    setActionLoading(true);
    try {
      await revokeApiKey(address, id);
      setModal({ type: "none" });
      await fetchKeys();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to revoke API key",
      );
    } finally {
      setActionLoading(false);
    }
  };

  if (!isConnected) {
    return (
      <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
          <div>
            <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
              Developer // API Keys
            </div>
            <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
              <Key className="w-8 h-8 text-accent" />
              API Keys
            </h2>
          </div>
        </div>
        <div className="text-center py-20">
          <Shield className="w-16 h-16 text-muted-foreground mx-auto mb-4" />
          <h3 className="text-xl font-bold text-foreground mb-2">
            Connect Your Wallet
          </h3>
          <p className="text-muted-foreground">
            Connect your Stellar wallet to manage API keys.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            Developer // API Keys
          </div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <Key className="w-8 h-8 text-accent" />
            API Keys
          </h2>
        </div>
        <p className="text-muted-foreground text-sm max-w-xl">
          Manage API keys for programmatic access to the Stellar Insights API.
        </p>
        <button
          onClick={() => setModal({ type: "create" })}
          className="flex items-center gap-2 px-4 py-2 bg-accent text-white rounded-xl hover:opacity-90 transition-opacity font-medium shrink-0"
        >
          <Plus className="w-4 h-4" />
          Create Key
        </button>
      </div>

      {error && (
        <div className="bg-destructive/10 border border-destructive/30 rounded-xl p-4 flex items-center gap-3">
          <AlertTriangle className="w-5 h-5 text-destructive shrink-0" />
          <p className="text-destructive text-sm">{error}</p>
          <button
            onClick={() => setError(null)}
            className="ml-auto text-destructive hover:opacity-80"
            title="Dismiss error"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {loading ? (
        <div className="glass rounded-xl p-8 border border-border">
          <div className="animate-pulse space-y-4">
            {[...Array(3)].map((_, i) => (
              <div
                key={i}
                className="h-12 bg-muted/50 rounded-lg"
              />
            ))}
          </div>
        </div>
      ) : keys.length === 0 ? (
        <div className="glass rounded-xl p-12 text-center border border-border">
          <Key className="w-12 h-12 text-muted-foreground mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-foreground mb-2">
            No API Keys
          </h3>
          <p className="text-muted-foreground mb-6 max-w-sm mx-auto">
            Create your first API key to start accessing the Stellar Insights
            API programmatically.
          </p>
          <button
            onClick={() => setModal({ type: "create" })}
            className="inline-flex items-center gap-2 px-4 py-2 bg-accent text-white rounded-xl hover:opacity-90 transition-opacity font-medium"
          >
            <Plus className="w-4 h-4" />
            Create Your First Key
          </button>
        </div>
      ) : (
        <div className="glass rounded-xl border border-border overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-border bg-muted/20">
                  <th className="text-left px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Name
                  </th>
                  <th className="text-left px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Key
                  </th>
                  <th className="text-left px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Scopes
                  </th>
                  <th className="text-left px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Status
                  </th>
                  <th className="text-left px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Created
                  </th>
                  <th className="text-left px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Last Used
                  </th>
                  <th className="text-right px-6 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {keys.map((key) => (
                  <KeyRow
                    key={key.id}
                    apiKey={key}
                    onRotate={() => setModal({ type: "rotate", key })}
                    onRevoke={() => setModal({ type: "revoke", key })}
                  />
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {modal.type === "create" && (
        <CreateKeyModal
          onClose={() => setModal({ type: "none" })}
          onCreate={handleCreate}
          loading={actionLoading}
        />
      )}

      {modal.type === "reveal" && (
        <RevealKeyModal
          response={modal.response}
          onClose={() => setModal({ type: "none" })}
        />
      )}

      {modal.type === "rotate" && (
        <ConfirmModal
          title="Rotate API Key"
          message={`This will revoke the key "${modal.key.name}" and generate a new one. Any applications using the current key will stop working.`}
          confirmLabel="Rotate Key"
          variant="warning"
          loading={actionLoading}
          onConfirm={() => handleRotate(modal.key.id)}
          onCancel={() => setModal({ type: "none" })}
        />
      )}

      {modal.type === "revoke" && (
        <ConfirmModal
          title="Revoke API Key"
          message={`This will permanently revoke the key "${modal.key.name}". This action cannot be undone.`}
          confirmLabel="Revoke Key"
          variant="danger"
          loading={actionLoading}
          onConfirm={() => handleRevoke(modal.key.id)}
          onCancel={() => setModal({ type: "none" })}
        />
      )}
    </div>
  );
}

function KeyRow({
  apiKey,
  onRotate,
  onRevoke,
}: {
  apiKey: ApiKeyInfo;
  onRotate: () => void;
  onRevoke: () => void;
}) {
  const isActive = apiKey.status === "active";

  return (
    <tr className="hover:bg-muted/10 transition-colors">
      <td className="px-6 py-4">
        <div className="flex items-center gap-2">
          <Key className="w-4 h-4 text-muted-foreground" />
          <span className="font-medium text-foreground">{apiKey.name}</span>
        </div>
      </td>
      <td className="px-6 py-4">
        <code className="text-sm bg-muted px-2 py-1 rounded font-mono text-foreground">
          {apiKey.key_prefix}
        </code>
      </td>
      <td className="px-6 py-4">
        <div className="flex gap-1 flex-wrap">
          {apiKey.scopes.split(",").map((scope) => (
            <span
              key={scope}
              className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-accent/20 text-accent"
            >
              {scope.trim()}
            </span>
          ))}
        </div>
      </td>
      <td className="px-6 py-4">
        <span
          className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
            isActive
              ? "bg-green-500/20 text-green-400"
              : "bg-destructive/20 text-destructive"
          }`}
        >
          {apiKey.status}
        </span>
      </td>
      <td className="px-6 py-4 text-sm text-muted-foreground">
        {formatDate(apiKey.created_at)}
      </td>
      <td className="px-6 py-4 text-sm text-muted-foreground">
        {apiKey.last_used_at ? formatDate(apiKey.last_used_at) : "Never"}
      </td>
      <td className="px-6 py-4">
        {isActive && (
          <div className="flex items-center justify-end gap-2">
            <button
              onClick={onRotate}
              title="Rotate key"
              className="p-1.5 text-muted-foreground hover:text-accent hover:bg-accent/10 rounded-lg transition-colors"
            >
              <RefreshCw className="w-4 h-4" />
            </button>
            <button
              onClick={onRevoke}
              title="Revoke key"
              className="p-1.5 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded-lg transition-colors"
            >
              <Trash2 className="w-4 h-4" />
            </button>
          </div>
        )}
      </td>
    </tr>
  );
}

function CreateKeyModal({
  onClose,
  onCreate,
  loading,
}: {
  onClose: () => void;
  onCreate: (name: string, scopes: string, expiresAt?: string) => void;
  loading: boolean;
}) {
  const [name, setName] = useState("");
  const [scopes, setScopes] = useState("read");
  const [hasExpiry, setHasExpiry] = useState(false);
  const [expiresAt, setExpiresAt] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    onCreate(name.trim(), scopes, hasExpiry && expiresAt ? expiresAt : undefined);
  };

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
      <div className="glass rounded-xl shadow-xl max-w-md w-full border border-border">
        <div className="flex items-center justify-between p-6 border-b border-border">
          <h2 className="text-lg font-semibold text-foreground">
            Create API Key
          </h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-lg"
            title="Close"
          >
            <X className="w-5 h-5 text-muted-foreground" />
          </button>
        </div>
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          <div>
            <label className="block text-sm font-medium text-foreground mb-1">
              Key Name
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Production Server"
              className="w-full bg-background border border-border rounded-lg px-4 py-2 text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-accent"
              required
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-foreground mb-1">
              Permissions
            </label>
            <select
              value={scopes}
              onChange={(e) => setScopes(e.target.value)}
              aria-label="Permissions"
              className="w-full bg-background border border-border rounded-lg px-4 py-2 text-foreground focus:outline-none focus:ring-2 focus:ring-accent"
            >
              <option value="read">Read Only</option>
              <option value="read,write">Read & Write</option>
              <option value="read,write,admin">Full Access</option>
            </select>
          </div>
          <div>
            <label className="flex items-center gap-2 text-sm text-foreground">
              <input
                type="checkbox"
                checked={hasExpiry}
                onChange={(e) => setHasExpiry(e.target.checked)}
                className="rounded border-border"
              />
              Set expiration date
            </label>
            {hasExpiry && (
              <input
                type="datetime-local"
                value={expiresAt}
                onChange={(e) => setExpiresAt(e.target.value)}
                aria-label="Expiration date"
                className="mt-2 w-full bg-background border border-border rounded-lg px-4 py-2 text-foreground focus:outline-none focus:ring-2 focus:ring-accent"
              />
            )}
          </div>
          <div className="flex justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-muted-foreground hover:bg-muted rounded-lg transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || !name.trim()}
              className="px-4 py-2 bg-accent text-white rounded-lg hover:opacity-90 transition-opacity font-medium disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? "Creating..." : "Create Key"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

function RevealKeyModal({
  response,
  onClose,
}: {
  response: CreateApiKeyResponse;
  onClose: () => void;
}) {
  const [copied, setCopied] = useState(false);
  const [visible, setVisible] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(response.plain_key);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      const el = document.createElement("textarea");
      el.value = response.plain_key;
      document.body.appendChild(el);
      el.select();
      document.execCommand("copy");
      document.body.removeChild(el);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
      <div className="glass rounded-xl shadow-xl max-w-lg w-full border border-border">
        <div className="flex items-center justify-between p-6 border-b border-border">
          <h2 className="text-lg font-semibold text-foreground">
            API Key Created
          </h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-lg"
            title="Close"
          >
            <X className="w-5 h-5 text-muted-foreground" />
          </button>
        </div>
        <div className="p-6 space-y-4">
          <div className="bg-amber-500/10 border border-amber-500/30 rounded-lg p-4 flex items-start gap-3">
            <AlertTriangle className="w-5 h-5 text-amber-500 shrink-0 mt-0.5" />
            <p className="text-sm text-amber-600 dark:text-amber-400">
              Copy this key now. You won&apos;t be able to see it again.
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium text-foreground mb-1">
              Key Name
            </label>
            <p className="font-medium text-foreground">{response.key.name}</p>
          </div>

          <div>
            <label className="block text-sm font-medium text-foreground mb-1">
              API Key
            </label>
            <div className="flex items-center gap-2">
              <code className="flex-1 bg-muted border border-border rounded-lg px-4 py-2.5 font-mono text-sm text-foreground break-all">
                {visible
                  ? response.plain_key
                  : response.plain_key.substring(0, 12) +
                    "••••••••••••••••••••••"}
              </code>
              <button
                onClick={() => setVisible(!visible)}
                className="p-2 hover:bg-muted rounded-lg transition-colors"
                title={visible ? "Hide key" : "Show key"}
              >
                {visible ? (
                  <EyeOff className="w-4 h-4 text-muted-foreground" />
                ) : (
                  <Eye className="w-4 h-4 text-muted-foreground" />
                )}
              </button>
              <button
                onClick={handleCopy}
                className="p-2 hover:bg-muted rounded-lg transition-colors"
                title="Copy to clipboard"
              >
                {copied ? (
                  <Check className="w-4 h-4 text-green-500" />
                ) : (
                  <Copy className="w-4 h-4 text-muted-foreground" />
                )}
              </button>
            </div>
          </div>

          <div className="flex justify-end pt-2">
            <button
              onClick={onClose}
              className="px-4 py-2 bg-accent text-white rounded-lg hover:opacity-90 transition-opacity font-medium"
            >
              Done
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function ConfirmModal({
  title,
  message,
  confirmLabel,
  variant,
  loading,
  onConfirm,
  onCancel,
}: {
  title: string;
  message: string;
  confirmLabel: string;
  variant: "warning" | "danger";
  loading: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const btnClass =
    variant === "danger"
      ? "bg-destructive hover:opacity-90"
      : "bg-amber-500 hover:opacity-90";

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
      <div className="glass rounded-xl shadow-xl max-w-md w-full border border-border">
        <div className="flex items-center justify-between p-6 border-b border-border">
          <h2 className="text-lg font-semibold text-foreground">{title}</h2>
          <button
            onClick={onCancel}
            className="p-1 hover:bg-muted rounded-lg"
            title="Close"
          >
            <X className="w-5 h-5 text-muted-foreground" />
          </button>
        </div>
        <div className="p-6 space-y-4">
          <div className="flex items-start gap-3">
            <AlertTriangle
              className={`w-5 h-5 shrink-0 mt-0.5 ${variant === "danger" ? "text-destructive" : "text-amber-500"}`}
            />
            <p className="text-muted-foreground text-sm">{message}</p>
          </div>
          <div className="flex justify-end gap-3 pt-2">
            <button
              onClick={onCancel}
              className="px-4 py-2 text-muted-foreground hover:bg-muted rounded-lg transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={onConfirm}
              disabled={loading}
              className={`px-4 py-2 text-white rounded-lg transition-opacity font-medium disabled:opacity-50 disabled:cursor-not-allowed ${btnClass}`}
            >
              {loading ? "Processing..." : confirmLabel}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function formatDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    return date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return dateStr;
  }
}
