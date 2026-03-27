import { CreateApiKeyResponse } from "@/lib/api-keys";
import { AlertTriangle, Check, Copy, Eye, EyeOff, X } from "lucide-react";
import { useState } from "react";

export default function RevealKeyModal({
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