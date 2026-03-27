import { ApiKeyInfo } from "@/lib/api-keys";
import { Key, RefreshCw, Trash2 } from "lucide-react";

export default function KeyRow({
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