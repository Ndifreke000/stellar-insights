"use client";

import React, { useState } from "react";
import { Download, X, FileJson, FileSpreadsheet, FileText, CheckCircle2, AlertCircle, Loader2 } from "lucide-react";

interface ExportDialogProps {
  isOpen: boolean;
  onClose: () => void;
  defaultType?: "corridors" | "anchors" | "payments";
}

export function ExportDialog({ isOpen, onClose, defaultType = "corridors" }: ExportDialogProps) {
  const [exportType, setExportType] = useState<"corridors" | "anchors" | "payments">(defaultType);
  const [format, setFormat] = useState<"csv" | "json" | "excel">("csv");
  const [dateRange, setDateRange] = useState("30d");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState<"idle" | "success" | "error">("idle");
  const [errorMessage, setErrorMessage] = useState("");

  if (!isOpen) return null;

  const handleExport = async () => {
    try {
      setLoading(true);
      setStatus("idle");
      setErrorMessage("");

      // Calculate dates
      const end = new Date();
      const start = new Date();
      if (dateRange === "7d") start.setDate(end.getDate() - 7);
      else if (dateRange === "30d") start.setDate(end.getDate() - 30);
      else if (dateRange === "90d") start.setDate(end.getDate() - 90);
      else start.setDate(end.getDate() - 365); // "all" ~ 1 year for safety

      const startDateStr = start.toISOString().split("T")[0];
      const endDateStr = end.toISOString().split("T")[0];

      // Build API URL
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";
      const url = `${apiUrl}/api/export/${exportType}?format=${format}&start_date=${startDateStr}&end_date=${endDateStr}`;

      const response = await fetch(url, {
        method: "GET",
      });

      if (!response.ok) {
        throw new Error(`Export failed: ${response.statusText}`);
      }

      // Handle streaming download to support large files and progress
      const disposition = response.headers.get('Content-Disposition');
      let filename = `${exportType}_export.${format === 'excel' ? 'xlsx' : format}`;
      
      if (disposition && disposition.indexOf('attachment') !== -1) {
          const filenameRegex = /filename[^;=\n]*=((['"]).*?\2|[^;\n]*)/;
          const matches = filenameRegex.exec(disposition);
          if (matches != null && matches[1]) { 
            filename = matches[1].replace(/['"]/g, '');
          }
      }

      const blob = await response.blob();
      const downloadUrl = window.URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = downloadUrl;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      window.URL.revokeObjectURL(downloadUrl);
      document.body.removeChild(a);

      setStatus("success");
      setTimeout(() => {
        onClose();
        setStatus("idle");
      }, 2000);

    } catch (err: any) {
      console.error(err);
      setStatus("error");
      setErrorMessage(err.message || "An error occurred during export");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="glass-card w-full max-w-md rounded-3xl border border-border/50 shadow-2xl overflow-hidden animate-in zoom-in-95 duration-200">
        
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-border/10 bg-slate-900/40">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-accent/10 rounded-xl">
              <Download className="w-5 h-5 text-accent" />
            </div>
            <div>
              <h3 className="text-lg font-bold uppercase tracking-tight">Export Data</h3>
              <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mt-1">
                Download historical metrics
              </p>
            </div>
          </div>
          <button 
            onClick={onClose}
            className="p-2 hover:bg-white/5 rounded-xl transition-colors text-muted-foreground hover:text-foreground"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 space-y-6">
          {/* Data Type Selection */}
          <div className="space-y-3">
            <label className="text-xs font-mono font-bold uppercase tracking-widest text-muted-foreground">Data Type</label>
            <div className="grid grid-cols-2 gap-2">
              <button
                onClick={() => setExportType("corridors")}
                className={`p-3 rounded-xl border text-sm font-semibold transition-all ${exportType === "corridors" ? "border-accent bg-accent/10 text-accent" : "border-border/50 bg-slate-900/50 text-muted-foreground hover:border-accent/30"}`}
              >
                Corridors
              </button>
              <button
                onClick={() => setExportType("anchors")}
                className={`p-3 rounded-xl border text-sm font-semibold transition-all ${exportType === "anchors" ? "border-accent bg-accent/10 text-accent" : "border-border/50 bg-slate-900/50 text-muted-foreground hover:border-accent/30"}`}
              >
                Anchors
              </button>
            </div>
          </div>

          {/* Date Range */}
          <div className="space-y-3">
            <label className="text-xs font-mono font-bold uppercase tracking-widest text-muted-foreground">Time Period</label>
            <select
              value={dateRange}
              onChange={(e) => setDateRange(e.target.value)}
              className="w-full bg-slate-900/50 border border-border/50 rounded-xl px-4 py-3 text-sm font-semibold focus:outline-none focus:ring-2 focus:ring-accent/50 appearance-none cursor-pointer"
            >
              <option value="7d">Last 7 Days</option>
              <option value="30d">Last 30 Days</option>
              <option value="90d">Last 90 Days</option>
              <option value="all">All Available</option>
            </select>
          </div>

          {/* Format */}
          <div className="space-y-3">
            <label className="text-xs font-mono font-bold uppercase tracking-widest text-muted-foreground">Format</label>
            <div className="grid grid-cols-3 gap-2">
              <button
                onClick={() => setFormat("csv")}
                className={`flex flex-col items-center gap-2 p-3 rounded-xl border transition-all ${format === "csv" ? "border-accent bg-accent/10 text-accent" : "border-border/50 bg-slate-900/50 text-muted-foreground hover:border-accent/30"}`}
              >
                <FileText className="w-5 h-5" />
                <span className="text-xs font-bold uppercase">CSV</span>
              </button>
              <button
                onClick={() => setFormat("excel")}
                className={`flex flex-col items-center gap-2 p-3 rounded-xl border transition-all ${format === "excel" ? "border-green-500 bg-green-500/10 text-green-500" : "border-border/50 bg-slate-900/50 text-muted-foreground hover:border-green-500/30"}`}
              >
                <FileSpreadsheet className="w-5 h-5" />
                <span className="text-xs font-bold uppercase">Excel</span>
              </button>
              <button
                onClick={() => setFormat("json")}
                className={`flex flex-col items-center gap-2 p-3 rounded-xl border transition-all ${format === "json" ? "border-yellow-500 bg-yellow-500/10 text-yellow-500" : "border-border/50 bg-slate-900/50 text-muted-foreground hover:border-yellow-500/30"}`}
              >
                <FileJson className="w-5 h-5" />
                <span className="text-xs font-bold uppercase">JSON</span>
              </button>
            </div>
          </div>

          {/* Status Region */}
          {status === "success" && (
            <div className="flex items-center gap-2 text-green-500 bg-green-500/10 p-3 rounded-lg border border-green-500/20 text-sm font-semibold animate-in zoom-in">
              <CheckCircle2 className="w-4 h-4" />
              Export completed successfully
            </div>
          )}

          {status === "error" && (
            <div className="flex items-center gap-2 text-red-500 bg-red-500/10 p-3 rounded-lg border border-red-500/20 text-sm font-semibold animate-in zoom-in">
              <AlertCircle className="w-4 h-4 flex-shrink-0" />
              {errorMessage}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-border/10 bg-slate-900/40 flex justify-end gap-3">
          <button
            onClick={onClose}
            className="px-5 py-2.5 rounded-xl text-sm font-semibold text-muted-foreground hover:text-foreground hover:bg-white/5 transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleExport}
            disabled={loading}
            className="flex items-center gap-2 px-6 py-2.5 rounded-xl text-sm font-bold text-white bg-accent hover:bg-accent/90 focus:ring-2 focus:ring-accent/50 focus:ring-offset-2 focus:ring-offset-background disabled:opacity-50 disabled:cursor-not-allowed transition-all glow-accent"
          >
            {loading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Download className="w-4 h-4" />
            )}
            {loading ? "Exporting..." : "Download Data"}
          </button>
        </div>
      </div>
    </div>
  );
}
