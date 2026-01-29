import React from "react";
import type { Metadata, Viewport } from "next";
import { ErrorBoundary } from "../components/ErrorBoundary";
import { MonitoringProvider } from "../components/MonitoringProvider";
import { WalletProvider } from "../components/lib/wallet-context";
import { NotificationProvider } from "../contexts/NotificationContext";
import { NotificationSystem } from "../components/notifications/NotificationSystem";
import "./globals.css";

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  userScalable: false,
};

export const metadata: Metadata = {
  title: "Stellar Insights - Payment Network Intelligence",
  description:
    "Deep insights into Stellar payment network performance. Predict success, optimize routing, quantify reliability, and identify liquidity bottlenecks.",
  icons: {
    icon: [
      {
        url: "/icon-light-32x32.png",
        media: "(prefers-color-scheme: light)",
      },
      {
        url: "/icon-dark-32x32.png",
        media: "(prefers-color-scheme: dark)",
      },
      {
        url: "/icon.svg",
        type: "image/svg+xml",
      },
    ],
    apple: "/apple-icon.png",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`font-sans antialiased`}>
        <ErrorBoundary>
          <WalletProvider>
            <NotificationProvider>
              {children}
              <NotificationSystem />
            </NotificationProvider>
          </WalletProvider>
        </ErrorBoundary>
        {/* <Analytics /> */}
      </body>
    </html>
  );
}
