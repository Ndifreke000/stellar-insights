import React from "react";
import type { Metadata, Viewport } from "next";
import { headers } from "next/headers";
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

export default async function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const headersList = await headers();
  const locale = headersList.get("x-next-intl-locale") ?? "en";

  return (
    <html lang={locale} className="dark" suppressHydrationWarning>
      <body
        className="font-sans antialiased text-foreground selection:bg-accent/30"
        suppressHydrationWarning
      >
        {children}
      </body>
    </html>
  );
}
