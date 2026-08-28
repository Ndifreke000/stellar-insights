"use client";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import { WifiOff, RefreshCw, Home, Clock, Database } from "lucide-react";
import { motion } from "framer-motion";

export default function OfflinePage() {
  const [retrying, setRetrying] = useState(false);
  const [offlineSince] = useState(() => new Date());
  const [elapsed, setElapsed] = useState("0s");

  // Live elapsed counter
  useEffect(() => {
    const id = setInterval(() => {
      const secs = Math.floor((Date.now() - offlineSince.getTime()) / 1000);
      if (secs < 60) setElapsed(`${secs}s`);
      else setElapsed(`${Math.floor(secs / 60)}m ${secs % 60}s`);
    }, 1000);
    return () => clearInterval(id);
  }, [offlineSince]);

  const handleRetry = () => {
    setRetrying(true);
    // Give the browser a moment to re-check connectivity before reloading
    setTimeout(() => window.location.reload(), 800);
  };

  const cachedRoutes = [
    { label: "Dashboard", href: "/en/dashboard", icon: Database },
    { label: "Corridors", href: "/en/corridors", icon: Database },
    { label: "Analytics", href: "/en/analytics", icon: Database },
  ];

  return (
    <div className="min-h-screen flex items-center justify-center p-6 bg-background">
      {/* Ambient glow */}
      <div className="fixed top-[-10%] left-[-10%] w-[40%] h-[40%] bg-accent/5 rounded-full blur-[120px] -z-10" />
      <div className="fixed bottom-[-10%] right-[-10%] w-[30%] h-[30%] bg-blue-500/5 rounded-full blur-[100px] -z-10" />

      <motion.div
        initial={{ opacity: 0, y: 24 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, ease: "easeOut" }}
        className="glass-card rounded-3xl p-8 max-w-md w-full space-y-8"
      >
        {/* Icon + heading */}
        <div className="text-center space-y-4">
          <motion.div
            animate={{ scale: [1, 1.05, 1] }}
            transition={{ repeat: Infinity, duration: 3, ease: "easeInOut" }}
            className="w-20 h-20 rounded-2xl bg-muted flex items-center justify-center mx-auto"
          >
            <WifiOff className="w-9 h-9 text-error" aria-hidden="true" />
          </motion.div>

          <div>
            <div className="text-[10px] font-mono text-error uppercase tracking-[0.2em] mb-2">
              Connection Lost
            </div>
            <h1 className="text-3xl font-black tracking-tighter uppercase italic">
              Offline Mode
            </h1>
            <p className="text-sm text-muted-foreground mt-2">
              No internet connection detected. Cached data is available below.
            </p>
          </div>
        </div>

        {/* Elapsed time */}
        <div className="flex items-center justify-center gap-2 text-xs font-mono text-muted-foreground">
          <Clock className="w-3.5 h-3.5" aria-hidden="true" />
          <span>Offline for <span className="text-foreground font-semibold">{elapsed}</span></span>
        </div>

        {/* What's available */}
        <div className="space-y-3">
          <p className="text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
            Cached Pages
          </p>
          <div className="space-y-2">
            {cachedRoutes.map((route) => (
              <a
                key={route.href}
                href={route.href}
                className="flex items-center gap-3 p-3 rounded-xl border border-border hover:border-accent/40 hover:bg-accent/5 transition-all group"
              >
                <div className="w-8 h-8 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                  <route.icon className="w-4 h-4 text-accent" aria-hidden="true" />
                </div>
                <span className="text-sm font-medium group-hover:text-accent transition-colors">
                  {route.label}
                </span>
                <span className="ml-auto text-[10px] font-mono text-muted-foreground uppercase tracking-wider">
                  Cached
                </span>
              </a>
            ))}
          </div>
        </div>

        {/* Notice */}
        <div className="text-xs text-muted-foreground bg-muted/40 rounded-xl p-3 space-y-1">
          <p><span className="text-success font-semibold">Available:</span> Charts, anchor data, recent metrics</p>
          <p><span className="text-warning font-semibold">Unavailable:</span> Real-time updates, new data</p>
        </div>

        {/* Actions */}
        <div className="flex gap-3">
          <button
            onClick={handleRetry}
            disabled={retrying}
            className="flex-1 flex items-center justify-center gap-2 py-3 rounded-2xl bg-accent text-white font-bold uppercase tracking-tight text-sm hover:bg-accent/90 active:scale-95 transition-all disabled:opacity-60"
          >
            <RefreshCw className={`w-4 h-4 ${retrying ? "animate-spin" : ""}`} aria-hidden="true" />
            {retrying ? "Retrying…" : "Retry"}
          </button>
          <Link
            href="/"
            className="flex-1 flex items-center justify-center gap-2 py-3 rounded-2xl border border-border hover:border-accent/40 font-bold uppercase tracking-tight text-sm transition-all"
          >
            <Home className="w-4 h-4" aria-hidden="true" />
            Home
          </Link>
        </div>
      </motion.div>
    </div>
  );
}
