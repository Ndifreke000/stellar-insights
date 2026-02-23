import React from "react";
import type { Metadata } from "next";
import { NextIntlClientProvider } from "next-intl";
import { getMessages, getTranslations, setRequestLocale } from "next-intl/server";
import { hasLocale } from "next-intl";
import { notFound } from "next/navigation";
import { routing } from "@/i18n/routing";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { MonitoringProvider } from "@/components/MonitoringProvider";
import { WalletProvider } from "@/components/lib/wallet-context";
import { NotificationProvider } from "@/contexts/NotificationContext";
import { ThemeProvider } from "@/contexts/ThemeContext";
import { UserPreferencesProvider } from "@/contexts/UserPreferencesContext";
import { NotificationSystem } from "@/components/notifications/NotificationSystem";
import { QuestProgressTracker } from "@/components/QuestProgressTracker";
import { Sidebar } from "@/components/layout/sidebar";
import { Navbar } from "@/components/navbar";
import { SkipNavigation } from "@/components/SkipNavigation";

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
            <WalletProvider>
              <NotificationProvider>
                <SkipNavigation />
                <div className="flex min-h-screen">
                  <Sidebar />
                  <main id="main-content" className="flex-1 ml-20 lg:ml-64 transition-all duration-300 relative">
                    <Navbar />

                    {/* Background Ambient Glow */}
                    <div className="fixed top-[-10%] left-[-10%] w-[40%] h-[40%] bg-accent/5 rounded-full blur-[120px] -z-10" aria-hidden="true" />
                    <div className="fixed bottom-[-10%] right-[-10%] w-[30%] h-[30%] bg-blue-500/5 rounded-full blur-[100px] -z-10" aria-hidden="true" />
                    <div className="fixed top-[-10%] left-[-10%] w-[40%] h-[40%] bg-accent/5 rounded-full blur-[120px] -z-10" aria-hidden="true" />
                    <div className="fixed bottom-[-10%] right-[-10%] w-[30%] h-[30%] bg-blue-500/5 rounded-full blur-[100px] -z-10" aria-hidden="true" />

                    <div className="p-4 md:p-8">{children}</div>
                  </main>
                </div>
                <QuestProgressTracker />
                <NotificationSystem />
              </NotificationProvider>
            </WalletProvider>
          </UserPreferencesProvider>
        </ThemeProvider>
      </ErrorBoundary>
    </NextIntlClientProvider>
  );
}
