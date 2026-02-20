"use client";

import React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  BarChart3,
  TrendingUp,
  Compass,
  Settings,
  Activity,
  Zap,
  ChevronLeft,
  ChevronRight,
  LayoutDashboard,
  Waves,
  Droplets,
  Users,
} from "lucide-react";
import { useState } from "react";

const navItems = [
  { name: "Home", icon: LayoutDashboard, path: "/" },
  { name: "Terminal", icon: LayoutDashboard, path: "/dashboard" },
  { name: "Corridors", icon: Compass, path: "/corridors" },
  { name: "Analytics", icon: BarChart3, path: "/analytics" },
  { name: "Trustlines", icon: Users, path: "/trustlines" },
  { name: "Network Health", icon: Activity, path: "/health" },
  { name: "Liquidity", icon: Waves, path: "/liquidity" },
  { name: "Pools", icon: Droplets, path: "/liquidity-pools" },
];

interface SidebarProps {
  open?: boolean;
  onClose?: () => void;
}

export function Sidebar({ open, onClose }: SidebarProps = {}) {
  const pathname = usePathname();
  const [collapsed, setCollapsed] = useState(false);

  return (
    <aside
      className={`fixed top-0 left-0 h-screen glass border-r border-border transition-all duration-500 z-50 ${
        collapsed ? "w-20" : "w-64"
      }`}
    >
      <div className="flex flex-col h-full">
        {/* Logo Section */}
        <div className="p-6 flex items-center gap-3">
          <div className="w-8 h-8 bg-accent rounded-lg flex items-center justify-center glow-accent shrink-0">
            <TrendingUp className="w-5 h-5 text-white" />
          </div>
          {!collapsed && (
            <span className="text-xl font-bold tracking-tighter text-foreground whitespace-nowrap overflow-hidden">
              STELLAR
              <span className="text-accent underline decoration-accent/30">
                INSIGHTS
              </span>
            </span>
          )}
        </div>

        {/* Navigation Section */}
        <nav className="flex-1 px-4 py-8 space-y-3">
          {navItems.map((item) => {
            const isActive = pathname === item.path;
            const Icon = item.icon;

            return (
              <Link
                key={item.path}
                href={item.path}
                className={`flex items-center gap-4 px-4 py-3 rounded-xl transition-all duration-300 group ${
                  isActive
                    ? "bg-accent/10 text-accent border border-accent/20"
                    : "text-muted-foreground hover:bg-white/5 hover:text-foreground border border-transparent"
                }`}
              >
                <Icon
                  className={`w-5 h-5 shrink-0 ${isActive ? "text-accent" : "group-hover:text-foreground"}`}
                />
                {!collapsed && (
                  <span className="font-bold text-sm uppercase tracking-widest">
                    {item.name}
                  </span>
                )}
                {isActive && !collapsed && (
                  <div className="ml-auto w-1 h-4 rounded-full bg-accent shadow-[0_0_8px_rgba(99,102,241,0.6)]" />
                )}
              </Link>
            );
          })}
        </nav>

        {/* Footer / Settings Section */}
        <div className="p-4 border-t border-border space-y-2">
          {!collapsed && (
            <div className="px-4 py-2 mb-2">
              <div className="flex items-center gap-2 mb-1">
                <div className="w-2 h-2 rounded-full bg-green-500 grow-success" />
                <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-tighter">
                  System Nominal
                </span>
              </div>
              <div className="text-[10px] font-mono text-muted-foreground/50 tabular-nums uppercase tracking-tighter">
                RPC_ID: STLR_MAIN_01
              </div>
            </div>
          )}

          <button
            onClick={() => setCollapsed(!collapsed)}
            className="w-full flex items-center gap-4 px-4 py-3 rounded-xl text-muted-foreground hover:bg-white/5 hover:text-foreground transition-all duration-300"
          >
            {collapsed ? (
              <ChevronRight className="w-5 h-5 shrink-0" />
            ) : (
              <ChevronLeft className="w-5 h-5 shrink-0" />
            )}
            {!collapsed && (
              <span className="text-xs font-bold uppercase tracking-widest">
                Collapse
              </span>
            )}
          </button>

          <Link
            href="/settings"
            className="flex items-center gap-4 px-4 py-3 rounded-xl text-muted-foreground hover:bg-white/5 hover:text-foreground transition-all duration-300"
          >
            <Settings className="w-5 h-5 shrink-0" />
            {!collapsed && (
              <span className="text-xs font-bold uppercase tracking-widest">
                Settings
              </span>
            )}
          </Link>
        </div>
      </div>
    </aside>
  );
}
