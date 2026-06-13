import { motion } from "framer-motion";
import { Activity, BellRing, Network, ShieldCheck, Sparkles, ArrowRight } from "lucide-react";

const features = [
  { icon: Activity, title: "Live Ledger Analytics", desc: "Stream ledger close times, fee charges, and operation throughput in real time." },
  { icon: Network, title: "Network Graph", desc: "Visualize account relationships and trustlines as an interactive force graph." },
  { icon: BellRing, title: "Smart Alerts", desc: "Get notified when whales move, anchors mint, or volatility crosses thresholds." },
  { icon: ShieldCheck, title: "Audit Trail", desc: "Verifiable, signed snapshots of network state for compliance and research." },
];

const stats = [
  { label: "Ledgers indexed", value: "62.4M" },
  { label: "Accounts tracked", value: "8.1M" },
  { label: "Alerts fired (24h)", value: "1,284" },
  { label: "Uptime", value: "99.98%" },
];

export default function App() {
  return (
    <main className="min-h-screen relative overflow-hidden">
      {/* Nav */}
      <header className="relative z-20 flex items-center justify-between px-6 md:px-12 py-6">
        <div className="flex items-center gap-2 font-display text-xl font-semibold">
          <div className="h-8 w-8 rounded-lg bg-gradient-primary shadow-glow" aria-hidden />
          <span>Awpwrate</span>
        </div>
        <nav className="hidden md:flex items-center gap-8 text-sm text-muted-foreground">
          <a href="#features" className="hover:text-foreground transition">Features</a>
          <a href="#stats" className="hover:text-foreground transition">Network</a>
          <a href="#cta" className="hover:text-foreground transition">Get access</a>
        </nav>
        <a href="#cta" className="hidden md:inline-flex items-center gap-2 text-sm font-medium px-4 py-2 rounded-full bg-foreground text-background hover:bg-primary hover:text-primary-foreground transition">
          Launch app <ArrowRight className="h-4 w-4" />
        </a>
      </header>

      {/* Hero */}
      <section className="relative z-10 px-6 md:px-12 pt-16 pb-32 max-w-7xl mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, ease: "easeOut" }}
          className="max-w-4xl"
        >
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full glass text-xs text-muted-foreground mb-8">
            <Sparkles className="h-3.5 w-3.5 text-primary" />
            Stellar network intelligence, in real time
          </div>
          <h1 className="text-5xl md:text-7xl lg:text-8xl font-bold leading-[0.95] mb-8">
            See the <span className="text-gradient">Stellar</span> network
            <br />
            like never before.
          </h1>
          <p className="text-lg md:text-xl text-muted-foreground max-w-2xl mb-10 leading-relaxed">
            Awpwrate ingests every ledger, decodes every operation, and surfaces the signals that
            matter — for traders, anchors, and protocol builders.
          </p>
          <div className="flex flex-wrap gap-4">
            <a href="#cta" className="inline-flex items-center gap-2 px-6 py-3 rounded-full bg-gradient-primary text-primary-foreground font-medium shadow-glow hover:scale-[1.02] transition">
              Start free trial <ArrowRight className="h-4 w-4" />
            </a>
            <a href="#features" className="inline-flex items-center gap-2 px-6 py-3 rounded-full glass font-medium hover:bg-card/60 transition">
              Explore features
            </a>
          </div>
        </motion.div>

        {/* Orbital decoration */}
        <div className="pointer-events-none absolute -right-40 top-10 h-[600px] w-[600px] rounded-full border border-primary/20 [mask-image:radial-gradient(white,transparent_70%)]" />
        <div className="pointer-events-none absolute -right-20 top-40 h-[400px] w-[400px] rounded-full border border-accent/20 [mask-image:radial-gradient(white,transparent_70%)]" />
      </section>

      {/* Stats */}
      <section id="stats" className="relative z-10 px-6 md:px-12 max-w-7xl mx-auto">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-px bg-border/50 rounded-2xl overflow-hidden glass">
          {stats.map((s) => (
            <div key={s.label} className="bg-background/60 p-6 md:p-8">
              <div className="font-mono text-3xl md:text-4xl text-gradient font-semibold">{s.value}</div>
              <div className="text-xs uppercase tracking-widest text-muted-foreground mt-2">{s.label}</div>
            </div>
          ))}
        </div>
      </section>

      {/* Features */}
      <section id="features" className="relative z-10 px-6 md:px-12 py-32 max-w-7xl mx-auto">
        <h2 className="text-4xl md:text-6xl font-bold mb-4 max-w-3xl">Built for everyone who lives on-chain.</h2>
        <p className="text-muted-foreground max-w-2xl mb-16 text-lg">A complete toolkit covering streaming analytics, graph exploration, alerting, and verifiable audits.</p>
        <div className="grid md:grid-cols-2 gap-6">
          {features.map((f, i) => (
            <motion.div
              key={f.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true }}
              transition={{ duration: 0.5, delay: i * 0.08 }}
              className="glass rounded-2xl p-8 hover:border-primary/40 transition group"
            >
              <div className="h-12 w-12 rounded-xl bg-gradient-primary/20 flex items-center justify-center mb-6 group-hover:shadow-glow transition">
                <f.icon className="h-6 w-6 text-primary" />
              </div>
              <h3 className="text-2xl font-semibold mb-2">{f.title}</h3>
              <p className="text-muted-foreground leading-relaxed">{f.desc}</p>
            </motion.div>
          ))}
        </div>
      </section>

      {/* CTA */}
      <section id="cta" className="relative z-10 px-6 md:px-12 pb-24 max-w-7xl mx-auto">
        <div className="glass rounded-3xl p-12 md:p-20 text-center relative overflow-hidden">
          <div className="absolute inset-0 bg-gradient-primary opacity-10" aria-hidden />
          <div className="relative">
            <h2 className="text-4xl md:text-6xl font-bold mb-6">Ready to read the chain?</h2>
            <p className="text-muted-foreground max-w-xl mx-auto mb-10 text-lg">Join researchers and trading desks already monitoring Stellar with Awpwrate.</p>
            <a href="#" className="inline-flex items-center gap-2 px-8 py-4 rounded-full bg-gradient-primary text-primary-foreground font-semibold shadow-glow hover:scale-[1.02] transition">
              Request access <ArrowRight className="h-4 w-4" />
            </a>
          </div>
        </div>
      </section>

      <footer className="relative z-10 px-6 md:px-12 py-10 border-t border-border/50 text-sm text-muted-foreground flex flex-wrap items-center justify-between gap-4 max-w-7xl mx-auto">
        <div>© {new Date().getFullYear()} Awpwrate. Stellar intelligence platform.</div>
        <div className="flex gap-6"><a href="#" className="hover:text-foreground">Docs</a><a href="#" className="hover:text-foreground">API</a><a href="#" className="hover:text-foreground">Status</a></div>
      </footer>
    </main>
  );
}