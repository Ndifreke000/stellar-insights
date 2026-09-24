import React from "react";
import type { Metadata } from "next";
import { NextIntlClientProvider } from "next-intl";
import { getMessages, getTranslations, setRequestLocale } from "next-intl/server";
import { hasLocale } from "next-intl";
import { notFound } from "next/navigation";
import { routing } from "@/i18n/routing";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { WalletProvider } from "@/components/lib/wallet-context";
import { NotificationProvider } from "@/contexts/NotificationContext";
import { ThemeProvider } from "@/contexts/ThemeContext";
import { UserPreferencesProvider } from "@/contexts/UserPreferencesContext";
import { KeyboardShortcutsProvider } from "@/contexts/KeyboardShortcutsContext";
import { NotificationSystem } from "@/components/notifications/NotificationSystem";
import { QuestProgressTracker } from "@/components/QuestProgressTracker";
import { ShortcutHelpOverlay } from "@/components/keyboard-shortcuts/ShortcutHelpOverlay";
import { ShortcutsInitializer } from "@/components/keyboard-shortcuts/ShortcutsInitializer";
import { OfflineBanner } from "@/components/OfflineBanner";
import { StateProvider } from "@/components/StateProvider";
import { CommandPaletteProvider } from "@/contexts/CommandPaletteContext";
import { CommandPalette } from "@/components/CommandPalette";
import { NetworkProvider } from "@/contexts/NetworkContext";
import { MobileAwareLayout } from "@/components/layout/MobileAwareLayout";

type Props = {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
};

export function generateStaticParams() {
  return routing.locales.map((locale) => ({ locale }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  if (!hasLocale(routing.locales, locale)) {
    return {};
  }
  const t = await getTranslations({ locale, namespace: "metadata" });
  return {
    title: t("title"),
    description: t("description"),
  };
}

export default async function LocaleLayout({ children, params }: Props) {
  const { locale } = await params;

  if (!hasLocale(routing.locales, locale)) {
    notFound();
  }

  setRequestLocale(locale);
  const messages = await getMessages();

  return (
    <NextIntlClientProvider messages={messages} locale={locale}>
      <ErrorBoundary>
        <ThemeProvider>
          <UserPreferencesProvider>
            <KeyboardShortcutsProvider>
              <CommandPaletteProvider>
              <WalletProvider>
                <NotificationProvider>
                  <StateProvider>
                    <NetworkProvider>
                    <OfflineBanner />
                    <ShortcutsInitializer />
                    <MobileAwareLayout>
                      {children}
                    </MobileAwareLayout>
                    <QuestProgressTracker />
                  <NotificationSystem />
                  <ShortcutHelpOverlay />
                  <CommandPalette />
                    </NetworkProvider>
                  </StateProvider>
                </NotificationProvider>
              </WalletProvider>
              </CommandPaletteProvider>
            </KeyboardShortcutsProvider>
          </UserPreferencesProvider>
        </ThemeProvider>
      </ErrorBoundary>
    </NextIntlClientProvider>
  );
}
