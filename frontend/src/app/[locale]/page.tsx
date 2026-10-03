"use client";

import { Link } from "@/i18n/navigation";
import { useTranslations } from "next-intl";
import {
  TrendingUp,
  Shield,
  Activity,
  ChevronRight,
  Globe,
  Database,
} from "lucide-react";
import { useWallet } from "@/components/lib/wallet-context";

export default function Home() {
  const { isConnected, connectWallet, isConnecting } = useWallet();
  const t = useTranslations("home");

  const steps = ["name", "check", "decide", "pay"] as const;

  return (
    <main className="space-y-24 pb-20">
      {/* Hero Section - Data First */}
      <section aria-labelledby="hero-heading" className="relative pt-12">
        <div className="max-w-5xl">
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-accent/10 border border-accent/20 mb-8 animate-pulse-slow" role="status" aria-live="polite">
            <Activity className="w-4 h-4 text-accent" aria-hidden="true" />
            <span className="text-xs font-semibold text-accent uppercase tracking-wider">
              {t("hero.liveBadge")}
            </span>
          </div>

          <h1 id="hero-heading" className="text-6xl md:text-7xl font-extrabold tracking-tighter leading-[1.1] mb-8">
            {t("hero.title")} <br />
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-accent to-blue-400">
              {t("hero.titleHighlight")}
            </span>
          </h1>

          <p className="text-xl text-muted-foreground max-w-2xl mb-10 leading-relaxed">
            {t("hero.subtitle")}
          </p>

          <div className="flex flex-wrap gap-4">
            <Link
              href="/dashboard"
              className="px-8 py-4 bg-accent text-white rounded-xl font-bold hover:glow-accent transition-all flex items-center gap-2 group"
            >
              {t("hero.enterTerminal")}{" "}
              <ChevronRight className="w-5 h-5 group-hover:translate-x-1 transition-transform" />
            </Link>
            {!isConnected && (
              <button
                onClick={connectWallet}
                disabled={isConnecting}
                className="px-8 py-4 glass text-foreground rounded-xl font-bold hover:bg-white/10 transition-all border-border"
              >
                {isConnecting ? t("hero.initializing") : t("hero.connectIdentity")}
              </button>
            )}
          </div>
        </div>
      </section>

      {/* How it works */}
      <section aria-label={t("steps.label")} className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {steps.map((step, i) => (
          <div key={step} className="glass-card p-6 rounded-2xl">
            <div className="text-xs font-mono font-bold text-accent mb-3">0{i + 1}</div>
            <h3 className="font-bold mb-2">{t(`steps.${step}.title`)}</h3>
            <p className="text-sm text-muted-foreground">{t(`steps.${step}.body`)}</p>
          </div>
        ))}
      </section>

      {/* Industrial Capabilities */}
      <section className="grid md:grid-cols-3 gap-8">
        <div className="space-y-4">
          <div className="w-12 h-12 rounded-xl glass flex items-center justify-center text-accent mb-6">
            <Globe className="w-6 h-6" />
          </div>
          <h3 className="text-2xl font-bold underline decoration-accent/30 underline-offset-8">
            {t("capabilities.globalCorridors")}
          </h3>
          <p className="text-muted-foreground leading-relaxed">
            {t("capabilities.globalCorridorsDesc")}
          </p>
        </div>

        <div className="space-y-4">
          <div className="w-12 h-12 rounded-xl glass flex items-center justify-center text-accent mb-6">
            <Database className="w-6 h-6" />
          </div>
          <h3 className="text-2xl font-bold underline decoration-accent/30 underline-offset-8">
            {t("capabilities.deepTelemetry")}
          </h3>
          <p className="text-muted-foreground leading-relaxed">
            {t("capabilities.deepTelemetryDesc")}
          </p>
        </div>

        <div className="space-y-4">
          <div className="w-12 h-12 rounded-xl glass flex items-center justify-center text-accent mb-6">
            <Shield className="w-6 h-6" />
          </div>
          <h3 className="text-2xl font-bold underline decoration-accent/30 underline-offset-8">
            {t("capabilities.predictiveTrust")}
          </h3>
          <p className="text-muted-foreground leading-relaxed">
            {t("capabilities.predictiveTrustDesc")}
          </p>
        </div>
      </section>

      {/* Bottom CTA */}
      <section className="glass-card p-12 rounded-[2rem] border border-accent/20 overflow-hidden relative group">
        <div className="absolute top-0 right-0 w-64 h-64 bg-accent/20 rounded-full blur-[80px] -z-10 group-hover:bg-accent/30 transition-colors" />
        <div className="max-w-2xl">
          <h2 className="text-4xl font-bold mb-6 tracking-tight">
            {t("cta.title")}
          </h2>
          <p className="text-lg text-muted-foreground mb-10">{t("cta.subtitle")}</p>
          <div className="flex gap-4">
            <Link
              href="/prediction"
              className="px-8 py-4 bg-accent text-white rounded-xl font-bold hover:scale-105 transition-transform"
            >
              {t("cta.launchTerminal")}
            </Link>
            <a
              href="https://github.com/Ndifreke000/stellar-insights/blob/main/docs/PLUGIN.md"
              target="_blank"
              rel="noreferrer"
              className="px-8 py-4 glass text-foreground rounded-xl font-bold hover:bg-white/10 transition-all"
            >
              {t("cta.readSpec")}
            </a>
          </div>
        </div>
      </section>

      <footer className="pt-12 border-t border-border flex flex-col md:flex-row justify-between items-center gap-6">
        <div className="flex items-center gap-2">
          <div className="w-6 h-6 bg-accent rounded flex items-center justify-center">
            <TrendingUp className="w-4 h-4 text-white" />
          </div>
          <span className="font-bold tracking-tight">{t("footer.payRaider")}</span>
        </div>
        <div className="flex gap-8 text-sm text-muted-foreground">
          <Link href="/corridors" className="hover:text-foreground transition-colors">
            {t("footer.networkStatus")}
          </Link>
          <Link href="/developer/keys" className="hover:text-foreground transition-colors">
            {t("footer.apiKeys")}
          </Link>
          <Link href="/governance" className="hover:text-foreground transition-colors">
            {t("footer.governance")}
          </Link>
        </div>
        <p className="text-xs text-muted-foreground/50 font-mono">
          {t("footer.copyright")}
        </p>
      </footer>
    </main>
  );
}
