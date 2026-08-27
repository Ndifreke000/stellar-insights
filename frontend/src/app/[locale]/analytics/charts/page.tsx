"use client";

import React from "react";
import dynamic from "next/dynamic";
import { ImageDown, ArrowLeft } from "lucide-react";
import { Link } from "@/i18n/navigation";
import { ErrorBoundary } from "@/components/ErrorBoundary";

const ChartExportGallery = dynamic(
  () =>
    import("@/components/charts/ChartExportGallery").then((m) => ({
      default: m.ChartExportGallery,
    })),
  { ssr: false }
);

export default function ChartExportPage() {
  return (
    <ErrorBoundary>
      <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
        {/* Page header */}
        <div className="flex flex-col sm:flex-row sm:items-end justify-between gap-4 border-b border-border/50 pb-6">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <Link
                href="/analytics"
                className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-white/5 transition-colors"
                aria-label="Back to Analytics"
              >
                <ArrowLeft className="w-4 h-4" aria-hidden="true" />
              </Link>
              <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em]">
                Analytics // Export Charts
              </div>
            </div>
            <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
              <ImageDown className="w-8 h-8 text-accent" aria-hidden="true" />
              Chart Export
            </h2>
            <p className="text-sm text-muted-foreground mt-2 max-w-lg">
              Download any chart as a high-resolution PNG or SVG image for use
              in reports, presentations, or external tools. Select one or more
              charts and batch-download them in one click.
            </p>
          </div>
        </div>

        <ChartExportGallery />
      </div>
    </ErrorBoundary>
  );
}
