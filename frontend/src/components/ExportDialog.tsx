"use client";

import React, { useState } from "react";
import { 
  Download, 
  X, 
  FileText, 
  Database, 
  Calendar, 
  CheckCircle2, 
  Loader2,
  AlertCircle,
  FileCode,
  Table
} from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { API_BASE_URL } from "@/lib/api";

interface ExportDialogProps {
  isOpen: boolean;
  onClose: () => void;
  type: "corridors" | "anchors" | "payments";
  title: string;
}

export function ExportDialog({ isOpen, onClose, type, title }: ExportDialogProps) {
  const [format, setFormat] = useState<"csv" | "json" | "excel">("csv");
  const [dateRange, setDateRange] = useState<"7d" | "30d" | "90d" | "custom">("30d");
  const [customStart, setCustomStart] = useState("");
  const [customEnd, setCustomEnd] = useState("");
  const [isExporting, setIsExporting] = useState(false);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const handleExport = async () => {
    setIsExporting(true);
    setProgress(10);
    setError(null);
    setSuccess(false);

    try {
      // Simulate progress for better UX
      const progressInterval = setInterval(() => {
        setProgress(prev => (prev < 90 ? prev + 5 : prev));
      }, 200);

      const params = new URLSearchParams();
      params.append("format", format === "excel" ? "xlsx" : format);
      
      if (dateRange === "custom" && customStart && customEnd) {
        params.append("start_date", new Date(customStart).toISOString());
        params.append("end_date", new Date(customEnd).toISOString());
      } else if (dateRange !== "custom") {
        const days = parseInt(dateRange);
        if (!isNaN(days)) {
          const start = new Date();
          start.setDate(start.getDate() - days);
          params.append("start_date", start.toISOString());
        }
      }

      const response = await fetch(`${API_BASE_URL}/export/${type}?${params.toString()}`);
      
      clearInterval(progressInterval);
      setProgress(100);

      if (!response.ok) {
        throw new Error(`Export failed: ${response.statusText}`);
      }

      const blob = await response.blob();
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${type}_export_${new Date().toISOString().split('T')[0]}.${format === "excel" ? "xlsx" : format}`;
      document.body.appendChild(a);
      a.click();
      window.URL.revokeObjectURL(url);
      document.body.removeChild(a);

      setSuccess(true);
      setTimeout(() => {
        onClose();
        // Reset state after closing
        setTimeout(() => {
          setIsExporting(false);
          setProgress(0);
          setSuccess(false);
        }, 300);
      }, 1500);

    } catch (err) {
      setError(err instanceof Error ? err.message : "An unknown error occurred");
      setIsExporting(false);
      setProgress(0);
    }
  };

  if (!isOpen) return null;

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          onClick={onClose}
          className="absolute inset-0 bg-slate-950/80 backdrop-blur-sm"
        />
        
        <motion.div
          initial={{ scale: 0.95, opacity: 0, y: 20 }}
          animate={{ scale: 1, opacity: 1, y: 0 }}
          exit={{ scale: 0.95, opacity: 0, y: 20 }}
          className="relative w-full max-w-md overflow-hidden glass-card rounded-3xl border border-white/10 shadow-2xl"
        >
          {/* Header */}
          <div className="flex items-center justify-between p-6 border-b border-white/5">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-accent/10 rounded-lg">
                <Download className="w-5 h-5 text-accent" />
              </div>
              <div>
                <h3 className="text-lg font-black tracking-tight uppercase italic text-white leading-none">
                  Data Export
                </h3>
                <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mt-1">
                  Target: {title}
                </p>
              </div>
            </div>
            <button
              onClick={onClose}
              className="p-2 hover:bg-white/5 rounded-full transition-colors text-muted-foreground hover:text-white"
            >
              <X className="w-5 h-5" />
            </button>
          </div>

          <div className="p-6 space-y-6">
            {/* Format Selection */}
            <div className="space-y-3">
              <label className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest flex items-center gap-2">
                <FileCode className="w-3 h-3" />
                Select Output Format
              </label>
              <div className="grid grid-cols-3 gap-2">
                {[
                  { id: "csv", label: "CSV", icon: FileText },
                  { id: "json", label: "JSON", icon: Database },
                  { id: "excel", label: "EXCEL", icon: Table },
                ].map((item) => (
                  <button
                    key={item.id}
                    onClick={() => setFormat(item.id as any)}
                    disabled={isExporting}
                    className={`flex flex-col items-center justify-center gap-2 p-4 rounded-xl border transition-all ${
                      format === item.id 
                        ? "bg-accent border-accent text-white shadow-[0_0_20px_rgba(var(--accent-rgb),0.3)]" 
                        : "bg-slate-900/50 border-white/5 text-muted-foreground hover:border-white/20 hover:text-white"
                    }`}
                  >
                    <item.icon className="w-5 h-5" />
                    <span className="text-[10px] font-bold uppercase tracking-tighter">{item.label}</span>
                  </button>
                ))}
              </div>
            </div>

            {/* Date Range Selection */}
            <div className="space-y-3">
              <label className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest flex items-center gap-2">
                <Calendar className="w-3 h-3" />
                Temporal Parameters
              </label>
              <div className="grid grid-cols-2 gap-2">
                {[
                  { id: "7d", label: "Last 7 Days" },
                  { id: "30d", label: "Last 30 Days" },
                  { id: "90d", label: "Last 90 Days" },
                  { id: "custom", label: "Custom Range" },
                ].map((range) => (
                  <button
                    key={range.id}
                    onClick={() => setDateRange(range.id as any)}
                    disabled={isExporting}
                    className={`px-4 py-3 rounded-xl border text-[10px] font-bold uppercase tracking-widest transition-all ${
                      dateRange === range.id 
                        ? "bg-white/10 border-white/20 text-white" 
                        : "bg-slate-900/50 border-white/5 text-muted-foreground hover:border-white/20 hover:text-white"
                    }`}
                  >
                    {range.label}
                  </button>
                ))}
              </div>
              
              {dateRange === "custom" && (
                <div className="animate-in fade-in slide-in-from-top-2 duration-300 grid grid-cols-2 gap-2 pt-2">
                  <div className="space-y-1">
                    <span className="text-[9px] font-mono text-muted-foreground uppercase ml-1">Start Date</span>
                    <input
                      type="date"
                      value={customStart}
                      onChange={(e) => setCustomStart(e.target.value)}
                      className="w-full bg-slate-950/50 border border-white/10 rounded-lg px-3 py-2 text-xs text-white focus:outline-none focus:ring-1 focus:ring-accent"
                    />
                  </div>
                  <div className="space-y-1">
                    <span className="text-[9px] font-mono text-muted-foreground uppercase ml-1">End Date</span>
                    <input
                      type="date"
                      value={customEnd}
                      onChange={(e) => setCustomEnd(e.target.value)}
                      className="w-full bg-slate-950/50 border border-white/10 rounded-lg px-3 py-2 text-xs text-white focus:outline-none focus:ring-1 focus:ring-accent"
                    />
                  </div>
                </div>
              )}
            </div>

            {/* Error Message */}
            {error && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                className="flex items-center gap-3 p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 text-xs"
              >
                <AlertCircle className="w-4 h-4 shrink-0" />
                <p>{error}</p>
              </motion.div>
            )}

            {/* Progress / Status */}
            {isExporting && (
              <div className="space-y-4 pt-2">
                <div className="flex justify-between items-end mb-1">
                  <span className="text-[10px] font-mono text-accent uppercase tracking-widest animate-pulse">
                    {success ? "Satellite Link Verified // 200" : "Extracting Telemetry Data..."}
                  </span>
                  <span className="text-[10px] font-mono text-white">{progress}%</span>
                </div>
                <div className="h-1.5 w-full bg-white/5 rounded-full overflow-hidden">
                  <motion.div
                    className="h-full bg-accent shadow-[0_0_10px_rgba(var(--accent-rgb),0.5)]"
                    initial={{ width: 0 }}
                    animate={{ width: `${progress}%` }}
                    transition={{ ease: "easeOut" }}
                  />
                </div>
              </div>
            )}
          </div>

          {/* Footer Actions */}
          <div className="p-6 pt-0">
            <button
              onClick={handleExport}
              disabled={isExporting || (dateRange === "custom" && (!customStart || !customEnd))}
              className={`w-full group relative overflow-hidden flex items-center justify-center gap-3 py-4 rounded-2xl font-black uppercase italic tracking-tighter transition-all ${
                isExporting 
                  ? "bg-slate-800 text-slate-500 cursor-not-allowed" 
                  : success
                    ? "bg-green-500 text-white"
                    : "bg-accent hover:bg-accent/90 text-white shadow-lg hover:shadow-accent/40 active:scale-95"
              }`}
            >
              {isExporting ? (
                <>
                  <Loader2 className="w-5 h-5 animate-spin" />
                  INITIATING SEQUENCE
                </>
              ) : success ? (
                <>
                  <CheckCircle2 className="w-5 h-5" />
                  DATA RECEIVED
                </>
              ) : (
                <>
                  <Download className="w-5 h-5 group-hover:translate-y-0.5 transition-transform" />
                  START EXPORT COMMAND
                </>
              )}
            </button>
            <p className="text-[9px] font-mono text-muted-foreground/40 text-center mt-4 uppercase tracking-[0.2em]">
              Security clearance level: ALPHA_VETA // 002
            </p>
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  );
}
