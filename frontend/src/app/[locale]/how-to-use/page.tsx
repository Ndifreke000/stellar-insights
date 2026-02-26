import React from "react";
import type { Metadata } from "next";
import {
 Rocket,
 BarChart3,
 Activity,
 Compass,
 Waves,
 Wallet,
 ChevronRight,
} from "lucide-react";
import { Link } from "@/i18n/navigation";


export const metadata: Metadata = {
 title: "How to Use — Stellar Insights",
 description:
   "A step-by-step guide to getting started with Stellar Insights — connect your wallet, explore corridors, and monitor network health.",
};


const steps = [
 {
   step: "01",
   icon: Wallet,
   title: "Connect Your Wallet",
   body: "Click the wallet icon in the sidebar to connect your Stellar wallet. This unlocks personalised corridor tracking and saved views.",
   cta: null,
 },
 {
   step: "02",
   icon: BarChart3,
   title: "Explore the Terminal",
   body: "Head to the Terminal dashboard for a global overview — live ledger close times, transaction throughput, and current fee pressure at a glance.",
   cta: { label: "Open Terminal", href: "/dashboard" },
 },
 {
   step: "03",
   icon: Compass,
   title: "Analyse Payment Corridors",
   body: "Navigate to Corridors to inspect any source-destination asset pair. Review historical success rates, median latency, and volume trends.",
   cta: { label: "View Corridors", href: "/corridors" },
 },
 {
   step: "04",
   icon: Activity,
   title: "Monitor Network Health",
   body: "The Network Health screen tracks validator quorum sets, ledger close variance, and surfaced anomalies in real time.",
   cta: { label: "Check Health", href: "/health" },
 },
 {
   step: "05",
   icon: Waves,
   title: "Inspect Liquidity Depth",
   body: "Use the Liquidity section to view order-book depth per asset pair, anchor reserve levels, and spread tightness over time.",
   cta: { label: "Liquidity", href: "/liquidity" },
 },
 {
   step: "06",
   icon: BarChart3,
   title: "Deep-Dive with Analytics",
   body: "Analytics lets you run custom queries across historical telemetry data, build comparison charts, and export CSV reports.",
   cta: { label: "Analytics", href: "/analytics" },
 },
];


export default function HowToUsePage() {
 return (
   <div className="max-w-4xl mx-auto space-y-14">
     {/* Hero */}
     <section className="text-center space-y-5 pt-4">
       <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-accent/30 bg-accent/10 text-accent text-xs font-semibold tracking-widest uppercase">
         <Rocket className="w-3.5 h-3.5" />
         Getting Started
       </div>


       <h1 className="text-4xl md:text-5xl font-extrabold tracking-tight text-foreground">
         How to Use{" "}
         <span className="text-accent">Stellar Insights</span>
       </h1>


       <p className="text-muted-foreground text-lg max-w-xl mx-auto leading-relaxed">
         Follow the steps below to unlock the full power of real-time Stellar network
         intelligence — from your first login to advanced corridor analytics.
       </p>
     </section>


     {/* Steps */}
     <section className="space-y-5">
       {steps.map((s, i) => {
         const Icon = s.icon;
         return (
           <div
             key={s.step}
             className="glass-card rounded-2xl p-6 border border-border hover:border-accent/25 transition-all duration-300 flex gap-5 group"
           >
             {/* Step number */}
             <div className="shrink-0 flex flex-col items-center gap-2">
               <div className="w-12 h-12 rounded-xl bg-accent/10 flex items-center justify-center group-hover:bg-accent/20 transition-colors">
                 <Icon className="w-5 h-5 text-accent" />
               </div>
               {i < steps.length - 1 && (
                 <div className="w-px flex-1 bg-accent/10 min-h-[20px]" />
               )}
             </div>


             {/* Content */}
             <div className="flex-1 space-y-2 pt-1">
               <div className="flex items-center gap-3">
                 <span className="text-xs font-mono text-accent/60 tracking-widest">
                   STEP {s.step}
                 </span>
               </div>
               <h2 className="text-lg font-bold text-foreground">{s.title}</h2>
               <p className="text-sm text-muted-foreground leading-relaxed">{s.body}</p>


               {s.cta && (
                 <Link
                   href={s.cta.href}
                   className="inline-flex items-center gap-1.5 mt-2 text-sm font-semibold text-accent hover:text-accent/80 transition-colors"
                 >
                   {s.cta.label}
                   <ChevronRight className="w-3.5 h-3.5" />
                 </Link>
               )}
             </div>
           </div>
         );
       })}
     </section>


     {/* Tips card */}
     <section className="glass-card rounded-2xl p-6 border border-accent/15 bg-accent/5 space-y-3">
       <p className="text-xs font-mono uppercase tracking-widest text-accent">Pro Tip</p>
       <p className="text-sm text-muted-foreground leading-relaxed">
         Enable browser notifications to receive real-time alerts when a monitored corridor
         drops below your success-rate threshold or when a major liquidity event is detected.
         You can configure alert thresholds on each corridor&apos;s detail page.
       </p>
     </section>


     <div className="pb-8" />
   </div>
 );
}
