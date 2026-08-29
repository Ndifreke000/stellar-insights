"use client";

import { useEffect, useRef } from "react";

declare global {
  interface Window {
    Redoc?: {
      init: (
        specUrl: string,
        options: Record<string, unknown>,
        element: HTMLElement | null,
      ) => void;
    };
  }
}

/**
 * Generated API docs rendered from the committed OpenAPI spec (docs/openapi.json).
 * Replaces the hand-rolled endpoint catalogue, playground, and examples pages.
 */
export default function ApiDocsPage() {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const script = document.createElement("script");
    script.src = "https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js";
    script.async = true;
    script.onload = () => {
      window.Redoc?.init("/api/openapi", {}, container);
    };
    document.body.appendChild(script);

    return () => {
      script.remove();
      container.replaceChildren();
    };
  }, []);

  return (
    <main className="min-h-screen bg-background">
      <div ref={containerRef} />
    </main>
  );
}
