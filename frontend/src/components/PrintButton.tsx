"use client";

import React, { useCallback } from "react";
import { Printer } from "lucide-react";

interface PrintButtonProps {
  /** Optional label; defaults to "Print" */
  label?: string;
  /** Extra Tailwind classes */
  className?: string;
  /** Called just before window.print() — use to prepare the DOM if needed */
  onBeforePrint?: () => void;
  /** Called after the print dialog closes */
  onAfterPrint?: () => void;
}

/**
 * A button that triggers the browser's native print dialog.
 * Works alongside the `@media print` styles in globals.css to produce
 * clean, printer-friendly output (issue #1488).
 */
export function PrintButton({
  label = "Print",
  className = "",
  onBeforePrint,
  onAfterPrint,
}: PrintButtonProps) {
  const handlePrint = useCallback(() => {
    onBeforePrint?.();

    // Add transition-suppression class so theme transitions don't fire
    document.documentElement.classList.remove("theme-transition");

    const afterPrint = () => {
      window.removeEventListener("afterprint", afterPrint);
      onAfterPrint?.();
    };
    window.addEventListener("afterprint", afterPrint);

    window.print();
  }, [onBeforePrint, onAfterPrint]);

  return (
    <button
      onClick={handlePrint}
      aria-label="Print this page"
      className={`print-hide flex items-center gap-2 px-4 py-2 rounded-xl border border-border bg-card text-sm font-semibold text-muted-foreground hover:text-foreground hover:border-accent/40 transition-all ${className}`}
    >
      <Printer className="w-4 h-4" aria-hidden="true" />
      {label}
    </button>
  );
}
