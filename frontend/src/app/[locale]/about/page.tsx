import React from "react";
import type { Metadata } from "next";
import { TrendingUp, Shield, Zap, Globe, BarChart3, Activity } from "lucide-react";


export const metadata: Metadata = {
 title: "About Us — Stellar Insights",
 description:
   "Learn about Stellar Insights — the team behind institutional-grade telemetry for the Stellar payment network.",
};


const pillars = [
 {
   icon: Shield,
   title: "Institutional Grade",
   body: "Built for funds, exchanges, and fintech infrastructure teams that need reliable, auditable network observability.",
 },
 {
   icon: Zap,
   title: "Real-Time Intelligence",
   body: "Sub-second telemetry streams delivered over WebSocket give you a live pulse on corridor health and settlement finality.",
 },
 {
   icon: Globe,
   title: "Global Corridor Coverage",
   body: "We track every active payment corridor on the Stellar network, from major fiat pairs to exotic asset routes.",
 },
 {
   icon: BarChart3,
   title: "Predictive Analytics",
   body: "Machine-learning models forecast transaction success rates and liquidity depth before you commit capital.",
 },
 {
   icon: Activity,
   title: "Network Health Monitoring",
   body: "Continuous validator monitoring, ledger close time tracking, and anomaly detection built into one dashboard.",
 },
 {
   icon: TrendingUp,
   title: "Liquidity Insights",
   body: "Surface order-book depth, spread dynamics, and anchor reserve levels across all active trading pairs.",
 },
];


export default function AboutPage() {
 return (
   <div className="max-w-5xl mx-auto space-y-16">
     {/* Hero */}
     <section className="text-center space-y-5 pt-4">
       <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-accent/30 bg-accent/10 text-accent text-xs font-semibold tracking-widest uppercase">
         <TrendingUp className="w-3.5 h-3.5" />
         About Stellar Insights
       </div>


       <h1 className="text-4xl md:text-5xl font-extrabold tracking-tight text-foreground leading-tight">
         Quantifying Trust on the{" "}
         <span className="text-accent">Stellar Network</span>
       </h1>


       <p className="text-muted-foreground text-lg max-w-2xl mx-auto leading-relaxed">
         Stellar Insights was created to give institutions, developers, and traders the
         high-fidelity telemetry they need to operate confidently on the world&apos;s fastest
         payment network. We move beyond simple transaction counts to surface what actually
         matters — liquidity depth, corridor reliability, and settlement certainty.
       </p>
     </section>



     <section className="glass-card rounded-2xl p-8 border border-accent/10 relative overflow-hidden">
       <div className="absolute inset-0 bg-gradient-to-br from-accent/5 to-transparent pointer-events-none" />
       <div className="relative space-y-3">
         <p className="text-xs font-mono uppercase tracking-widest text-accent">Our Mission</p>
         <h2 className="text-2xl font-bold text-foreground">
           Make network intelligence accessible to every builder on Stellar.
         </h2>
         <p className="text-muted-foreground leading-relaxed">
           We believe that transparent, real-time insight into payment flows is a public good.
           By surfacing latency, liquidity, and failure patterns, we help the ecosystem route
           value more efficiently and trust the network more fully.
         </p>
       </div>
     </section>



     <section className="space-y-6">
       <h2 className="text-2xl font-bold text-foreground">What We Provide</h2>
       <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
         {pillars.map((p) => {
           const Icon = p.icon;
           return (
             <div
               key={p.title}
               className="glass-card rounded-xl p-6 border border-border hover:border-accent/30 transition-all duration-300 group space-y-3"
             >
               <div className="w-10 h-10 rounded-lg bg-accent/10 flex items-center justify-center group-hover:bg-accent/20 transition-colors">
                 <Icon className="w-5 h-5 text-accent" />
               </div>
               <h3 className="font-bold text-foreground">{p.title}</h3>
               <p className="text-sm text-muted-foreground leading-relaxed">{p.body}</p>
             </div>
           );
         })}
       </div>
     </section>



     <section className="text-center pb-8">
       <p className="text-sm text-muted-foreground/60 font-mono">
         RPC_ID: STLR_MAIN_01 &nbsp;·&nbsp; Version 1.0.0 &nbsp;·&nbsp; Network: Mainnet
       </p>
     </section>
   </div>
 );
}
