import { AlertTriangle, X } from "lucide-react";

export default function ConfirmModal({
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