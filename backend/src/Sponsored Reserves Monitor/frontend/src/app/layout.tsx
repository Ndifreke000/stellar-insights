// This is a placeholder for the frontend layout
// Real implementation would be in app/layout.tsx for Next.js 13+

import React from "react";
import "../globals.css";

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
