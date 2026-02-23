"use client";

import React, { useState } from "react";
import { useTranslations } from "next-intl";
import { Mail, Github, Twitter, Send, CheckCircle2, Loader2 } from "lucide-react";

const channelsConfig = [
  { icon: Mail, key: "email", value: "hello@stellarinsights.io", href: "mailto:hello@stellarinsights.io", hintKey: "emailHint" },
  { icon: Github, key: "github", value: "github.com/stellar-insights", href: "https://github.com/stellar-insights", hintKey: "githubHint" },
  { icon: Twitter, key: "twitter", value: "@StellarInsights", href: "https://twitter.com/StellarInsights", hintKey: "twitterHint" },
];

type FormState = "idle" | "sending" | "sent" | "error";

export default function ContactPage() {
  const t = useTranslations("contact");
  const [form, setForm] = useState({ name: "", email: "", subject: "", message: "" });
  const [status, setStatus] = useState<FormState>("idle");

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>
  ) => setForm((prev) => ({ ...prev, [e.target.name]: e.target.value }));

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setStatus("sending");

    await new Promise((r) => setTimeout(r, 1600));
    setStatus("sent");
  };

  return (
    <div className="max-w-5xl mx-auto space-y-14">
      <section className="text-center space-y-5 pt-4">
        <div className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full border border-accent/30 bg-accent/10 text-accent text-xs font-semibold tracking-widest uppercase">
          <Mail className="w-3.5 h-3.5" />
          {t("title")}
        </div>

        <h1 className="text-4xl md:text-5xl font-extrabold tracking-tight text-foreground">
          {t("headingBefore")}
          <span className="text-accent">{t("headingHighlight")}</span>
        </h1>

        <p className="text-muted-foreground text-lg max-w-xl mx-auto leading-relaxed">
          {t("subtitle")}
        </p>
      </section>

      <div className="grid grid-cols-1 lg:grid-cols-5 gap-8">
        <aside className="lg:col-span-2 space-y-4">
          <h2 className="text-sm font-bold uppercase tracking-widest text-muted-foreground mb-2">
            {t("otherChannels")}
          </h2>
          {channelsConfig.map((c) => {
            const Icon = c.icon;
            return (
              <a
                key={c.key}
                href={c.href}
                target="_blank"
                rel="noopener noreferrer"
                className="group flex items-start gap-4 glass-card rounded-xl p-4 border border-border hover:border-accent/30 transition-all duration-300"
              >
                <div className="w-10 h-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0 group-hover:bg-accent/20 transition-colors">
                  <Icon className="w-5 h-5 text-accent" />
                </div>
                <div>
                  <p className="font-semibold text-foreground text-sm">{t(`channels.${c.key}`)}</p>
                  <p className="text-accent text-sm">{c.value}</p>
                  <p className="text-muted-foreground text-xs mt-0.5">{t(c.hintKey)}</p>
                </div>
              </a>
            );
          })}
        </aside>

        <div className="lg:col-span-3 glass-card rounded-2xl p-7 border border-border">
          {status === "sent" ? (
            <div className="h-full flex flex-col items-center justify-center text-center gap-4 py-12">
              <CheckCircle2 className="w-14 h-14 text-green-400" />
              <h2 className="text-xl font-bold text-foreground">{t("form.messageSent")}</h2>
              <p className="text-muted-foreground text-sm max-w-xs">
                {t("form.thanks")}
              </p>
              <button
                onClick={() => { setStatus("idle"); setForm({ name: "", email: "", subject: "", message: "" }); }}
                className="mt-2 text-accent text-sm font-semibold hover:underline"
              >
                {t("form.sendAnother")}
              </button>
            </div>
          ) : (
            <form onSubmit={handleSubmit} className="space-y-5">
              <h2 className="text-lg font-bold text-foreground mb-1">{t("form.sectionTitle")}</h2>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <label htmlFor="contact-name" className="contact-label">{t("form.name")}</label>
                  <input
                    id="contact-name"
                    name="name"
                    required
                    placeholder={t("form.placeholders.name")}
                    value={form.name}
                    onChange={handleChange}
                    className="contact-input"
                  />
                </div>
                <div className="space-y-1.5">
                  <label htmlFor="contact-email" className="contact-label">{t("form.email")}</label>
                  <input
                    id="contact-email"
                    name="email"
                    type="email"
                    required
                    placeholder={t("form.placeholders.email")}
                    value={form.email}
                    onChange={handleChange}
                    className="contact-input"
                  />
                </div>
              </div>

              <div className="space-y-1.5">
                <label htmlFor="contact-subject" className="contact-label">{t("form.subject")}</label>
                <select
                  id="contact-subject"
                  name="subject"
                  required
                  value={form.subject}
                  onChange={handleChange}
                  className="contact-input"
                >
                  <option value="" disabled>{t("form.selectTopic")}</option>
                  <option value="support">{t("form.options.support")}</option>
                  <option value="bug">{t("form.options.bug")}</option>
                  <option value="feature">{t("form.options.feature")}</option>
                  <option value="partnership">{t("form.options.partnership")}</option>
                  <option value="other">{t("form.options.other")}</option>
                </select>
              </div>

              <div className="space-y-1.5">
                <label htmlFor="contact-message" className="contact-label">{t("form.message")}</label>
                <textarea
                  id="contact-message"
                  name="message"
                  required
                  rows={5}
                  placeholder={t("form.placeholders.message")}
                  value={form.message}
                  onChange={handleChange}
                  className="contact-input resize-none"
                />
              </div>

              <button
                type="submit"
                disabled={status === "sending"}
                className="w-full flex items-center justify-center gap-2 px-6 py-3 rounded-xl bg-accent text-white font-bold text-sm tracking-wide hover:bg-accent/90 active:scale-[0.98] transition-all disabled:opacity-60 disabled:cursor-not-allowed shadow-[0_0_20px_rgba(99,102,241,0.3)]"
              >
                {status === "sending" ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    {t("form.sending")}
                  </>
                ) : (
                  <>
                    <Send className="w-4 h-4" />
                    {t("form.sendMessage")}
                  </>
                )}
              </button>
            </form>
          )}
        </div>
      </div>

      <div className="pb-8" />
    </div>
  );
}
