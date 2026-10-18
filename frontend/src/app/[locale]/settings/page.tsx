"use client";

import React from "react";
import Link from "next/link";
import { useTranslations } from "next-intl";
import { LanguageSwitcher } from "@/components/LanguageSwitcher";
import { ThemeCustomizer } from "@/components/ThemeCustomizer";
import { ShortcutCustomizer } from "@/components/keyboard-shortcuts/ShortcutCustomizer";
import { ProgressiveWebApp } from "@/components/ProgressiveWebApp";
import { useOfflineStatus } from "@/hooks/useOfflineStatus";
import { WifiOff, Wifi, ShieldCheck } from "lucide-react";

function OfflineStatusBadge() {
  const { isOnline, offlineSince } = useOfflineStatus();
  return (
    <div
      className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-semibold ${
        isOnline
          ? "bg-success/10 text-success"
          : "bg-error/10 text-error"
      }`}
    >
      {isOnline ? (
        <Wifi className="w-3.5 h-3.5" aria-hidden="true" />
      ) : (
        <WifiOff className="w-3.5 h-3.5" aria-hidden="true" />
      )}
      {isOnline
        ? "Online"
        : `Offline${offlineSince ? ` since ${offlineSince.toLocaleTimeString()}` : ""}`}
    </div>
  );
}

export default function SettingsPage() {
  const t = useTranslations("layout.sidebar");

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      <h1 className="text-3xl font-bold">{t("settings")}</h1>

      {/* Theme customization */}
      <div className="glass-card p-6 rounded-2xl">
        <h2 className="text-lg font-semibold mb-6">Theme &amp; Appearance</h2>
        <ThemeCustomizer />
      </div>

      {/* Language */}
      <div className="glass-card p-6 rounded-2xl">
        <h2 className="text-lg font-semibold mb-4">Language</h2>
        <LanguageSwitcher />
      </div>

      {/* Offline / PWA */}
      <div className="glass-card p-6 rounded-2xl space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">Offline &amp; Installation</h2>
          <OfflineStatusBadge />
        </div>
        <p className="text-sm text-muted-foreground">
          Install PayRaider as an app for instant access and offline support.
          Cached data remains available for up to 24 hours without a connection.
        </p>
        <ProgressiveWebApp
          showInstallPrompt
          showOfflineIndicator
          showUpdateNotification
          showCacheManagement
        />
      </div>

      {/* Keyboard shortcuts */}
      <div className="glass-card p-6 rounded-2xl">
        <ShortcutCustomizer />
      </div>

      {/* Privacy & GDPR */}
      <div className="glass-card p-6 rounded-2xl">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Privacy &amp; Data</h2>
            <p className="text-sm text-muted-foreground mt-1">
              Manage your consents, export your data, or request deletion under GDPR.
            </p>
          </div>
          <Link
            href="settings/gdpr"
            className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white text-sm font-medium transition-colors"
          >
            <ShieldCheck className="w-4 h-4" aria-hidden="true" />
            Privacy Settings
          </Link>
        </div>
      </div>
    </div>
  );
}
