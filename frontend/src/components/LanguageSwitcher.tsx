"use client";

import React, { useTransition } from "react";
import { useLocale, useTranslations } from "next-intl";
import { useRouter, usePathname } from "@/i18n/navigation";
import { useUserPreferences } from "@/contexts/UserPreferencesContext";
import type { Locale } from "@/contexts/UserPreferencesContext";

const LOCALE_LABELS: Record<Locale, string> = {
  en: "EN",
  es: "ES",
  zh: "中文",
};

const LOCALES: Locale[] = ["en", "es", "zh"];

export function LanguageSwitcher() {
  const locale = useLocale() as Locale;
  const router = useRouter();
  const pathname = usePathname();
  const [isPending, startTransition] = useTransition();
  const { setPrefs } = useUserPreferences();

  const handleChange = (newLocale: Locale) => {
    if (newLocale === locale) return;
    startTransition(() => {
      setPrefs({ locale: newLocale });
      document.cookie = `NEXT_LOCALE=${newLocale};path=/;max-age=31536000;SameSite=Lax`;
      router.replace(pathname, { locale: newLocale });
    });
  };

  return (
    <div className="flex items-center gap-1 rounded-lg border border-border bg-background/50 p-1">
      {LOCALES.map((loc) => (
        <button
          key={loc}
          type="button"
          onClick={() => handleChange(loc)}
          disabled={isPending}
          className={`min-w-[2rem] px-2 py-1 text-xs font-bold uppercase rounded transition-colors ${
            locale === loc
              ? "bg-accent text-white"
              : "text-muted-foreground hover:text-foreground hover:bg-white/5"
          } ${isPending ? "opacity-70 cursor-not-allowed" : ""}`}
          aria-label={`Switch to ${LOCALE_LABELS[loc]}`}
        >
          {LOCALE_LABELS[loc]}
        </button>
      ))}
    </div>
  );
}
