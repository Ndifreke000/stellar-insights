"use client";

import React, { useState } from "react";
import { Header } from "./header";
import { Sidebar } from "./sidebar";
import { BottomNav } from "./bottom-nav";
import { SkipNavigation } from "../SkipNavigation";

interface LayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: LayoutProps) {
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const handleMenuToggle = () => {
    setSidebarOpen(!sidebarOpen);
  };

  const handleSidebarClose = () => {
    setSidebarOpen(false);
  };

  return (
    <>
      {/* Skip Navigation Links */}
      <SkipNavigation />

      <div className="min-h-screen bg-gray-50 dark:bg-slate-950">
        <Header onMenuToggle={handleMenuToggle} sidebarOpen={sidebarOpen} />
        <Sidebar open={sidebarOpen} onClose={handleSidebarClose} />

        {/* Main Content with proper landmark */}
        <main 
          id="main-content" 
          className="pt-16 lg:pl-64 pb-16 lg:pb-0"
          role="main"
          aria-label="Main content"
        >
          <div className="min-h-[calc(100vh-128px)] lg:min-h-[calc(100vh-64px)]">
            {children}
          </div>
        </main>

        {/* Bottom Navigation for Mobile */}
        <BottomNav />
      </div>
    </>
  );
}
