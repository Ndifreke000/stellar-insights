import type { CSSProperties } from "react";
import { useCorridor } from "../hooks/useCorridor.js";

export interface CorridorReliabilityCardProps {
  source: string;
  destination: string;
  className?: string;
}

const cardStyle: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "0.5rem",
  padding: "1rem",
  borderRadius: "0.5rem",
  border: "1px solid #2a2f3a",
  background: "#0f1115",
  color: "#e6e8eb",
  fontFamily: "system-ui, sans-serif",
  minWidth: "220px",
};

/**
 * Minimal, dependency-free widget showing a corridor's success rate,
 * average latency, and volume. Intentionally unstyled beyond inline
 * defaults so it drops into any host app without a CSS/Tailwind
 * dependency — override via `className` if the host app has its own
 * design system.
 */
export function CorridorReliabilityCard({
  source,
  destination,
  className,
}: CorridorReliabilityCardProps) {
  const { data, error, isLoading } = useCorridor(source, destination);

  if (isLoading) {
    return (
      <div className={className} style={cardStyle} role="status" aria-live="polite">
        Loading {source}/{destination}…
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className={className} style={cardStyle} role="alert">
        Failed to load {source}/{destination}
        {error ? `: ${error.message}` : ""}
      </div>
    );
  }

  return (
    <div className={className} style={cardStyle}>
      <strong>
        {data.source}/{data.destination}
      </strong>
      <span>Success rate: {(data.success_rate * 100).toFixed(2)}%</span>
      <span>Avg latency: {data.avg_latency_ms.toFixed(0)}ms</span>
      <span>Volume: ${data.volume_usd.toLocaleString()}</span>
    </div>
  );
}
