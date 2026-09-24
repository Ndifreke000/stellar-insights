"use client";

import React from "react";
import { motion } from "framer-motion";
import { Sun, Moon, Monitor, Check, Palette, Type } from "lucide-react";
import { useTheme, type ThemePreference } from "@/contexts/ThemeContext";
import { useUserPreferences } from "@/contexts/UserPreferencesContext";

// ─── Accent colour presets ────────────────────────────────────────────────────
const ACCENT_PRESETS = [
  { label: "Indigo", value: "#6366f1" },
  { label: "Violet", value: "#8b5cf6" },
  { label: "Sky", value: "#0ea5e9" },
  { label: "Emerald", value: "#10b981" },
  { label: "Rose", value: "#f43f5e" },
  { label: "Amber", value: "#f59e0b" },
] as const;

// ─── Font-size presets ────────────────────────────────────────────────────────
const FONT_SIZE_PRESETS = [
  { label: "Small", value: "sm", px: "13px" },
  { label: "Default", value: "md", px: "14px" },
  { label: "Large", value: "lg", px: "16px" },
] as const;

type FontSizeValue = (typeof FONT_SIZE_PRESETS)[number]["value"];

// ─── Theme option card ────────────────────────────────────────────────────────
interface ThemeCardProps {
  preference: ThemePreference;
  label: string;
  icon: React.ElementType;
  active: boolean;
  onClick: () => void;
  preview: { bg: string; card: string; text: string };
}

function ThemeCard({ preference: _preference, label, icon: Icon, active, onClick, preview }: ThemeCardProps) {
  return (
    <button
      onClick={onClick}
      aria-pressed={active}
      aria-label={`Set theme to ${label}`}
      className={`relative flex flex-col gap-3 p-4 rounded-2xl border-2 transition-all cursor-pointer w-full text-left ${
        active
          ? "border-accent bg-accent/10 shadow-[0_0_20px_rgba(99,102,241,0.2)]"
          : "border-border hover:border-accent/40 bg-card"
      }`}
    >
      {/* Mini preview */}
      <div
        className="w-full h-16 rounded-xl overflow-hidden flex flex-col gap-1 p-2"
        style={{ background: preview.bg }}
        aria-hidden="true"
      >
        <div className="w-full h-2 rounded-full" style={{ background: preview.card }} />
        <div className="w-3/4 h-2 rounded-full" style={{ background: preview.text, opacity: 0.6 }} />
        <div className="w-1/2 h-2 rounded-full" style={{ background: preview.text, opacity: 0.3 }} />
      </div>

      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Icon className="w-4 h-4 text-muted-foreground" aria-hidden="true" />
          <span className="text-sm font-semibold">{label}</span>
        </div>
        {active && (
          <motion.div
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            className="w-5 h-5 rounded-full bg-accent flex items-center justify-center"
          >
            <Check className="w-3 h-3 text-white" aria-hidden="true" />
          </motion.div>
        )}
      </div>
    </button>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────
export function ThemeCustomizer() {
  const { themePreference, setThemePreference } = useTheme();
  const { prefs, setPrefs } = useUserPreferences();

  // Accent colour stored in user prefs; apply to CSS variable on change
  const accentColor: string = (prefs as unknown as Record<string, unknown>).accentColor as string ?? "#6366f1";
  const fontSize: FontSizeValue = ((prefs as unknown as Record<string, unknown>).fontSize as FontSizeValue) ?? "md";

  const applyAccent = (color: string) => {
    document.documentElement.style.setProperty("--accent", color);
    setPrefs({ accentColor: color } as unknown as Partial<typeof prefs>);
  };

  const applyFontSize = (size: FontSizeValue) => {
    const preset = FONT_SIZE_PRESETS.find((p) => p.value === size);
    if (preset) {
      document.documentElement.style.setProperty("--base-font-size", preset.px);
      document.documentElement.setAttribute("data-font-size", size);
    }
    setPrefs({ fontSize: size } as unknown as Partial<typeof prefs>);
  };

  const themeOptions: Array<{
    preference: ThemePreference;
    label: string;
    icon: typeof Sun;
    preview: { bg: string; card: string; text: string };
  }> = [
    {
      preference: "light",
      label: "Light",
      icon: Sun,
      preview: { bg: "#f8fafc", card: "#e2e8f0", text: "#0f172a" },
    },
    {
      preference: "dark",
      label: "Dark",
      icon: Moon,
      preview: { bg: "#020617", card: "#1e293b", text: "#f8fafc" },
    },
    {
      preference: "system",
      label: "System",
      icon: Monitor,
      preview: { bg: "linear-gradient(135deg,#020617 50%,#f8fafc 50%)", card: "#6366f1", text: "#94a3b8" },
    },
  ];

  return (
    <div className="space-y-8">
      {/* ── Theme mode ── */}
      <section aria-labelledby="theme-mode-heading">
        <div className="flex items-center gap-2 mb-4">
          <Palette className="w-4 h-4 text-accent" aria-hidden="true" />
          <h3 id="theme-mode-heading" className="text-sm font-semibold uppercase tracking-widest text-muted-foreground">
            Appearance
          </h3>
        </div>
        <div className="grid grid-cols-3 gap-3">
          {themeOptions.map((opt) => (
            <ThemeCard
              key={opt.preference}
              preference={opt.preference}
              label={opt.label}
              icon={opt.icon}
              active={themePreference === opt.preference}
              onClick={() => setThemePreference(opt.preference)}
              preview={opt.preview}
            />
          ))}
        </div>
      </section>

      {/* ── Accent colour ── */}
      <section aria-labelledby="accent-colour-heading">
        <div className="flex items-center gap-2 mb-4">
          <span className="w-4 h-4 rounded-full bg-accent" aria-hidden="true" />
          <h3 id="accent-colour-heading" className="text-sm font-semibold uppercase tracking-widest text-muted-foreground">
            Accent Colour
          </h3>
        </div>
        <div className="flex flex-wrap gap-3" role="radiogroup" aria-label="Accent colour">
          {ACCENT_PRESETS.map((preset) => {
            const isActive = accentColor === preset.value;
            return (
              <button
                key={preset.value}
                role="radio"
                aria-checked={isActive}
                aria-label={preset.label}
                onClick={() => applyAccent(preset.value)}
                className={`relative w-9 h-9 rounded-full border-2 transition-all hover:scale-110 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-accent ${
                  isActive ? "border-white scale-110 shadow-lg" : "border-transparent"
                }`}
                style={{ background: preset.value }}
                title={preset.label}
              >
                {isActive && (
                  <Check className="absolute inset-0 m-auto w-4 h-4 text-white" aria-hidden="true" />
                )}
              </button>
            );
          })}
        </div>
      </section>

      {/* ── Font size ── */}
      <section aria-labelledby="font-size-heading">
        <div className="flex items-center gap-2 mb-4">
          <Type className="w-4 h-4 text-accent" aria-hidden="true" />
          <h3 id="font-size-heading" className="text-sm font-semibold uppercase tracking-widest text-muted-foreground">
            Text Size
          </h3>
        </div>
        <div className="flex gap-3" role="radiogroup" aria-label="Text size">
          {FONT_SIZE_PRESETS.map((preset) => {
            const isActive = fontSize === preset.value;
            return (
              <button
                key={preset.value}
                role="radio"
                aria-checked={isActive}
                onClick={() => applyFontSize(preset.value)}
                className={`flex-1 py-3 rounded-xl border-2 text-sm font-semibold transition-all ${
                  isActive
                    ? "border-accent bg-accent/10 text-accent"
                    : "border-border hover:border-accent/40 text-muted-foreground"
                }`}
                style={{ fontSize: preset.px }}
              >
                {preset.label}
              </button>
            );
          })}
        </div>
      </section>
    </div>
  );
}
