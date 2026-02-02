"use client";

import Link from "next/link";
import {
  ArrowRight,
  TrendingUp,
  Zap,
  BarChart3,
  Shield,
  Activity,
  ChevronRight,
  Globe,
  Database
} from "lucide-react";
import { useState, useEffect } from "react";
import { useWallet } from "../components/lib/wallet-context";

export default function Home() {
  const { isConnected, connectWallet, isConnecting } = useWallet();

  const mockTickers = [
    { label: 'USDC/BRL', value: '0.998', change: '+0.02%', status: 'optimal' },
    { label: 'XLM/EUR', value: '3.1s', change: '-120ms', status: 'optimal' },
    { label: 'Network TVL', value: '$45.2M', change: '+5.5%', status: 'optimal' },
    { label: 'Success Rate', value: '99.98%', change: '+0.1%', status: 'optimal' },
  ];

  return (
    <div className="space-y-24 pb-20">
      {/* Hero Section - Data First */}
      <section className="relative pt-12">
        <div className="max-w-5xl">
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-accent/10 border border-accent/20 mb-8 animate-pulse-slow">
            <Activity className="w-4 h-4 text-accent" />
            <span className="text-xs font-semibold text-accent uppercase tracking-wider">Live Network Intelligence Active</span>
          </div>

          <h1 className="text-6xl md:text-7xl font-extrabold tracking-tighter leading-[1.1] mb-8">
            QUANTIFYING <br />
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-accent to-blue-400">
              NETWORK TRUST
            </span>
          </h1>

          <p className="text-xl text-muted-foreground max-w-2xl mb-10 leading-relaxed">
            Stellar Insights provides institutional-grade telemetry for the Stellar payment network.
            Move beyond transaction counting; analyze liquidity depth, corridor reliability,
            and settlement finality in real-time.
          </p>

          <div className="flex flex-wrap gap-4">
            <Link
              href="/dashboard"
              className="px-8 py-4 bg-accent text-white rounded-xl font-bold hover:glow-accent transition-all flex items-center gap-2 group"
            >
              Enter Terminal <ChevronRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
            </Link>
            {!isConnected && (
              <button
                onClick={connectWallet}
                disabled={isConnecting}
                className="px-8 py-4 glass text-foreground rounded-xl font-bold hover:bg-white/10 transition-all border-border"
              >
                {isConnecting ? 'Initializing...' : 'Connect Identity'}
              </button>
            )}
          </div>
        </div>
      </section>

      {/* Live Intelligence Strip */}
      <section className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {mockTickers.map((ticker, i) => (
          <div key={i} className="glass-card p-6 rounded-2xl group hover:border-accent/30 transition-colors">
            <div className="flex justify-between items-start mb-4">
              <span className="text-xs font-bold text-muted-foreground uppercase tracking-widest">{ticker.label}</span>
              <div className="w-2 h-2 rounded-full bg-green-500 glow-success" />
            </div>
            <div className="flex items-end gap-3">
              <span className="text-2xl font-mono font-bold">{ticker.value}</span>
              <span className="text-xs font-mono text-green-400 mb-1">{ticker.change}</span>
            </div>
          </div>
        ))}
      </section>

      {/* Industrial Capabilities */}
      <section className="grid md:grid-cols-3 gap-8">
        <div className="space-y-4">
          <div className="w-12 h-12 rounded-xl glass flex items-center justify-center text-accent mb-6">
            <Globe className="w-6 h-6" />
          </div>
          <h3 className="text-2xl font-bold underline decoration-accent/30 underline-offset-8">Global Corridors</h3>
          <p className="text-muted-foreground leading-relaxed">
            Detailed health analysis of cross-border payment paths. Monitor uptime, liquidity bottlenecks,
            and anchor performance across 50+ fiat pairs.
          </p>
        </div>

        <div className="space-y-4">
          <div className="w-12 h-12 rounded-xl glass flex items-center justify-center text-accent mb-6">
            <Database className="w-6 h-6" />
          </div>
          <h3 className="text-2xl font-bold underline decoration-accent/30 underline-offset-8">Deep Telemetry</h3>
          <p className="text-muted-foreground leading-relaxed">
            Beyond surface-level TPS. We analyze ledger-level settlement data to provide true speed-to-finality
            distribution for payment operations.
          </p>
        </div>

        <div className="space-y-4">
          <div className="w-12 h-12 rounded-xl glass flex items-center justify-center text-accent mb-6">
            <Shield className="w-6 h-6" />
          </div>
          <h3 className="text-2xl font-bold underline decoration-accent/30 underline-offset-8">Predictive Trust</h3>
          <p className="text-muted-foreground leading-relaxed">
            Our ML engine predicts corridor success probability, helping you route high-value payments through
            the most resilient network nodes.
          </p>
        </div>
      </section>

      {/* Bottom CTA */}
      <section className="glass-card p-12 rounded-[2rem] border border-accent/20 overflow-hidden relative group">
        <div className="absolute top-0 right-0 w-64 h-64 bg-accent/20 rounded-full blur-[80px] -z-10 group-hover:bg-accent/30 transition-colors" />
        <div className="max-w-2xl">
          <h2 className="text-4xl font-bold mb-6 tracking-tight">Ready to integrate network intelligence?</h2>
          <p className="text-lg text-muted-foreground mb-10">
            Join the forward-thinking anchors and wallets already using Stellar Insights to optimize
            their liquidity management and payment reliability.
          </p>
          <div className="flex gap-4">
            <Link href="/dashboard" className="px-8 py-4 bg-accent text-white rounded-xl font-bold hover:scale-105 transition-transform">
              Launch Terminal
            </Link>
            <button className="px-8 py-4 glass text-foreground rounded-xl font-bold hover:bg-white/10 transition-all">
              Read Specification
            </button>
          </div>
        </div>
      </section>

      <footer className="pt-12 border-t border-border flex flex-col md:flex-row justify-between items-center gap-6">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 bg-accent rounded flex items-center justify-center">
            <TrendingUp className="w-4 h-4 text-white" />
          </div>
          <span className="font-bold tracking-tight">Stellar Insights</span>
        </div>
        <div className="flex gap-8 text-sm text-muted-foreground">
          <a href="#" className="hover:text-foreground transition-colors">Network Status</a>
          <a href="#" className="hover:text-foreground transition-colors">API Keys</a>
          <a href="#" className="hover:text-foreground transition-colors">Governance</a>
        </div>
        <p className="text-xs text-muted-foreground/50 font-mono">
          © 2026 STELLAR_INSIGHTS // v1.4.0-STABLE
        </p>
      </footer>
    </div>
  );
}
