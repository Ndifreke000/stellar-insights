"use client";

import React from "react";
import { useTranslations } from "next-intl";
import { LanguageSwitcher } from "@/components/LanguageSwitcher";
import { ThemeToggle } from "@/components/ThemeToggle";
import { ShortcutCustomizer } from "@/components/keyboard-shortcuts/ShortcutCustomizer";

export default function SettingsPage() {
  const t = useTranslations("layout.sidebar");

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      <h1 className="text-3xl font-bold">{t("settings")}</h1>
      
      <div className="glass-card p-6 rounded-2xl space-y-6">
        <div>
          <h2 className="text-lg font-semibold mb-2">Language</h2>
          <LanguageSwitcher />
        </div>
        <div>
          <h2 className="text-lg font-semibold mb-2">Theme</h2>
          <ThemeToggle />
        </div>
      </div>

      <div className="glass-card p-6 rounded-2xl">
        <ShortcutCustomizer />
      </div>
    </div>
  );
}
