import React from "react";
import type { Metadata, Viewport } from "next";
import { ErrorBoundary } from "../components/ErrorBoundary";
import { MonitoringProvider } from "../components/MonitoringProvider";
import { WalletProvider } from "../components/lib/wallet-context";
import { NotificationProvider } from "../contexts/NotificationContext";
import { ThemeProvider } from "../contexts/ThemeContext";
import { NotificationSystem } from "../components/notifications/NotificationSystem";
import { Sidebar } from "../components/layout/sidebar";
import { Navbar } from "../components/navbar";
import "./globals.css";

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  userScalable: false,
};

export const metadata: Metadata = {
  title: "Stellar Insights - Payment Network Intelligence",
  description:
    "Institutional-grade insights into Stellar payment network performance. Predict success, optimize routing, and monitor liquidity.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark" suppressHydrationWarning>
      <body className="font-sans antialiased text-foreground selection:bg-accent/30" suppressHydrationWarning>
        <ErrorBoundary>
          <ThemeProvider>
            <WalletProvider>
              <NotificationProvider>
                <div className="flex min-h-screen">
                  <Sidebar />
                  <main className="flex-1 ml-20 lg:ml-64 transition-all duration-300 relative">

                    <Navbar />

                    {/* Background Ambient Glow */}
                    <div className="fixed top-[-10%] left-[-10%] w-[40%] h-[40%] bg-accent/5 rounded-full blur-[120px] -z-10" />
                    <div className="fixed bottom-[-10%] right-[-10%] w-[30%] h-[30%] bg-blue-500/5 rounded-full blur-[100px] -z-10" />

                    <div className="p-4 md:p-8">
                      {children}
                    </div>
                  </main>
                </div>
                <NotificationSystem />
              </NotificationProvider>
            </WalletProvider>
          </ThemeProvider>
        </ErrorBoundary>
      </body>
    </html>
  );
}
