"use client";

import React, { useState } from "react";
import { Mail, Github, Twitter, Send, CheckCircle2, Loader2 } from "lucide-react";


const channels = [
  {
    icon: Mail,
    label: "Email",
    value: "hello@stellarinsights.io",
    href: "mailto:hello@stellarinsights.io",
    hint: "Response within 24 hours",
  },
  {
    icon: Github,
    label: "GitHub",
    value: "github.com/stellar-insights",
    href: "https://github.com/stellar-insights",
    hint: "Open issues & feature requests",
  },
  {
    icon: Twitter,
    label: "Twitter / X",
    value: "@StellarInsights",
    href: "https://twitter.com/StellarInsights",
    hint: "News & quick support",
  },
];

type FormState = "idle" | "sending" | "sent" | "error";

export default function ContactPage() {
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
          Contact Us
        </div>

        <h1 className="text-4xl md:text-5xl font-extrabold tracking-tight text-foreground">
          We&apos;d Love to{" "}
          <span className="text-accent">Hear From You</span>
        </h1>

        <p className="text-muted-foreground text-lg max-w-xl mx-auto leading-relaxed">
          Have a question, found a bug, or want to partner with us?
          Reach out via the form below or any of our channels.
        </p>
      </section>

      <div className="grid grid-cols-1 lg:grid-cols-5 gap-8">

        <aside className="lg:col-span-2 space-y-4">
          <h2 className="text-sm font-bold uppercase tracking-widest text-muted-foreground mb-2">
            Other Channels
          </h2>
          {channels.map((c) => {
            const Icon = c.icon;
            return (
              <a
                key={c.label}
                href={c.href}
                target="_blank"
                rel="noopener noreferrer"
                className="group flex items-start gap-4 glass-card rounded-xl p-4 border border-border hover:border-accent/30 transition-all duration-300"
              >
                <div className="w-10 h-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0 group-hover:bg-accent/20 transition-colors">
                  <Icon className="w-5 h-5 text-accent" />
                </div>
                <div>
                  <p className="font-semibold text-foreground text-sm">{c.label}</p>
                  <p className="text-accent text-sm">{c.value}</p>
                  <p className="text-muted-foreground text-xs mt-0.5">{c.hint}</p>
                </div>
              </a>
            );
          })}
        </aside>

      
        <div className="lg:col-span-3 glass-card rounded-2xl p-7 border border-border">
          {status === "sent" ? (
            <div className="h-full flex flex-col items-center justify-center text-center gap-4 py-12">
              <CheckCircle2 className="w-14 h-14 text-green-400" />
              <h2 className="text-xl font-bold text-foreground">Message Sent!</h2>
              <p className="text-muted-foreground text-sm max-w-xs">
                Thanks for reaching out. We&apos;ll get back to you within 24 hours.
              </p>
              <button
                onClick={() => { setStatus("idle"); setForm({ name: "", email: "", subject: "", message: "" }); }}
                className="mt-2 text-accent text-sm font-semibold hover:underline"
              >
                Send another message
              </button>
            </div>
          ) : (
            <form onSubmit={handleSubmit} className="space-y-5">
              <h2 className="text-lg font-bold text-foreground mb-1">Send a Message</h2>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <label htmlFor="contact-name" className="contact-label">Name</label>
                  <input
                    id="contact-name"
                    name="name"
                    required
                    placeholder="Jane Doe"
                    value={form.name}
                    onChange={handleChange}
                    className="contact-input"
                  />
                </div>
                <div className="space-y-1.5">
                  <label htmlFor="contact-email" className="contact-label">Email</label>
                  <input
                    id="contact-email"
                    name="email"
                    type="email"
                    required
                    placeholder="jane@example.com"
                    value={form.email}
                    onChange={handleChange}
                    className="contact-input"
                  />
                </div>
              </div>

              <div className="space-y-1.5">
                <label htmlFor="contact-subject" className="contact-label">Subject</label>
                <select
                  id="contact-subject"
                  name="subject"
                  required
                  value={form.subject}
                  onChange={handleChange}
                  className="contact-input"
                >
                  <option value="" disabled>Select a topic…</option>
                  <option value="support">Technical Support</option>
                  <option value="bug">Bug Report</option>
                  <option value="feature">Feature Request</option>
                  <option value="partnership">Partnership</option>
                  <option value="other">Other</option>
                </select>
              </div>

              <div className="space-y-1.5">
                <label htmlFor="contact-message" className="contact-label">Message</label>
                <textarea
                  id="contact-message"
                  name="message"
                  required
                  rows={5}
                  placeholder="Tell us how we can help…"
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
                    Sending…
                  </>
                ) : (
                  <>
                    <Send className="w-4 h-4" />
                    Send Message
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
