"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Key,
  Plus,
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
import KeyRow from "../components/KeyRow";
import CreateKeyModal from "../components/CreateKeyModal";
import RevealKeyModal from "../components/RevealKeyModal";
import ConfirmModal from "../components/ConfirmModal";
import UpgradePanel from "../components/UpgradePanel";

type ModalState =
  | { type: "none" }
  | { type: "create" }
  | { type: "reveal"; response: CreateApiKeyResponse }
  | { type: "rotate"; key: ApiKeyInfo }
  | { type: "revoke"; key: ApiKeyInfo };

export default function DeveloperKeysPage() {
  // Keys belong to the wallet proven by its SEP-10 session.
  const { isConnected, address, isAuthenticated, authToken, authenticateWithSep10 } = useWallet();
  const [signingIn, setSigningIn] = useState(false);
  const [keys, setKeys] = useState<ApiKeyInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalState>({ type: "none" });
  const [actionLoading, setActionLoading] = useState(false);

  const fetchKeys = useCallback(async () => {
    if (!authToken) return;
    setLoading(true);
    setError(null);
    try {
      const response = await listApiKeys(authToken);
      setKeys(response.keys);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load API keys");
    } finally {
      setLoading(false);
    }
  }, [authToken]);

  useEffect(() => {
    if (isConnected && authToken) {
      fetchKeys();
    } else {
      setLoading(false);
    }
  }, [isConnected, authToken, fetchKeys]);

  const handleCreate = async (
    name: string,
    scopes: string,
    expiresAt?: string,
  ) => {
    if (!authToken) return;
    setActionLoading(true);
    try {
      const response = await createApiKey(authToken, name, scopes, expiresAt);
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
    if (!authToken) return;
    setActionLoading(true);
    try {
      const response = await rotateApiKey(authToken, id);
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
    if (!authToken) return;
    setActionLoading(true);
    try {
      await revokeApiKey(authToken, id);
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

  if (isConnected && !isAuthenticated) {
    return (
      <div className="text-center py-20 space-y-4">
        <Shield className="w-16 h-16 text-muted-foreground mx-auto" />
        <h3 className="text-xl font-bold text-foreground">Sign in with your wallet</h3>
        <p className="text-muted-foreground max-w-md mx-auto">
          API keys belong to a Stellar wallet. Sign a one-time login message to prove you
          control {address ? `${address.slice(0, 6)}…${address.slice(-4)}` : "it"}.
        </p>
        {error && <p role="alert" className="text-destructive text-sm">{error}</p>}
        <button
          type="button"
          disabled={signingIn}
          onClick={async () => {
            setSigningIn(true);
            setError(null);
            try {
              await authenticateWithSep10();
            } catch (err) {
              setError(err instanceof Error ? err.message : "Sign-in failed");
            } finally {
              setSigningIn(false);
            }
          }}
          className="inline-flex items-center gap-2 px-4 py-2 bg-accent text-white rounded-xl hover:opacity-90 font-medium disabled:opacity-60"
        >
          {signingIn ? "Waiting for wallet…" : "Sign in"}
        </button>
      </div>
    );
  }

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
          Manage API keys for programmatic access to the PayRaider API.
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
            aria-label="Dismiss error message"
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
            Create your first API key to start accessing the PayRaider
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

      {authToken && address && (
        <UpgradePanel authToken={authToken} address={address} keys={keys} />
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