"use client";

import React from "react";
import { Link, usePathname } from "@/i18n/navigation";
import { useTranslations } from "next-intl";
import {
  BarChart3,
  TrendingUp,
  Compass,
  Settings,
  Activity,
  ChevronLeft,
  ChevronRight,
  LayoutDashboard,
  Waves,
  Droplets,
  Users,
  Database,
  Calculator,
  Key,
  Trophy,
  ScrollText,
  Share2,
  Shield,
} from "lucide-react";
import { useUserPreferences } from "@/contexts/UserPreferencesContext";
import { LanguageSwitcher } from "@/components/LanguageSwitcher";

const navItems = [
  { key: "home", icon: LayoutDashboard, path: "/" },
  { key: "terminal", icon: LayoutDashboard, path: "/dashboard" },
  { key: "corridors", icon: Compass, path: "/corridors" },
  { key: "network", icon: Share2, path: "/network" },
  { key: "analytics", icon: BarChart3, path: "/analytics" },
  { key: "apiUsage", icon: Activity, path: "/analytics/api" },
  { key: "trustlines", icon: Users, path: "/trustlines" },
  { key: "networkHealth", icon: Activity, path: "/health" },
  { key: "liquidity", icon: Waves, path: "/liquidity" },
  { key: "pools", icon: Droplets, path: "/liquidity-pools" },
  { key: "sep6", icon: Database, path: "/sep6" },
  { key: "calculator", icon: Calculator, path: "/calculator" },
  { key: "apiKeys", icon: Key, path: "/developer/keys" },
  { key: "quests", icon: Trophy, path: "/quests" },
  { key: "governance", icon: ScrollText, path: "/governance" },
  { key: "privacy", icon: Shield, path: "/settings/gdpr" },
  { key: "alerts", icon: Activity, path: "/alerts" },
];

interface SidebarProps {
  open?: boolean;
  onClose?: () => void;
}

export function Sidebar({ open, onClose }: SidebarProps = {}) {
  const pathname = usePathname();
  const t = useTranslations("layout.sidebar");
  const { prefs, setPrefs } = useUserPreferences();
  const collapsed = prefs.sidebarCollapsed;
  const setCollapsed = (val: boolean) => setPrefs({ sidebarCollapsed: val });

  return (
    <aside
      aria-label="Sidebar navigation"
      className={`hidden md:block fixed top-0 left-0 h-screen overflow-y-auto glass border-r border-border transition-all duration-500 z-50 ${collapsed ? "w-20" : "w-64"
        }`}
    >
      <div className="flex flex-col h-full">
        {/* Logo Section */}
        <div className="p-6 flex items-center gap-3">
          <div className="w-8 h-8 bg-accent rounded-lg flex items-center justify-center glow-accent shrink-0" aria-hidden="true">
            <TrendingUp className="w-5 h-5 text-white" aria-hidden="true" />
          </div>
          {!collapsed && (
            <span className="text-xl font-bold tracking-tighter text-foreground whitespace-nowrap overflow-hidden">
              STELLAR
              <span className="text-accent underline decoration-accent/30">
                {" "}{t("stellarInsights")}
              </span>
            </span>
          )}
        </div>

        {/* Navigation Section */}
        <nav aria-label="Primary navigation" className="flex-1 px-4 py-8 space-y-3 overflow-y-auto">
          {navItems.map((item) => {
            const isActive = pathname === item.path;
            const Icon = item.icon;

            return (
              <Link
                key={item.path}
                href={item.path}
                aria-current={isActive ? "page" : undefined}
                aria-label={t(item.key)}
                className={`flex items-center gap-4 px-4 py-3 rounded-xl transition-all duration-300 group ${isActive
                    ? "bg-accent/10 text-accent border border-accent/20"
                    : "text-muted-foreground hover:bg-white/5 hover:text-foreground border border-transparent"
                  }`}
              >
                <Icon
                  aria-hidden="true"
                  className={`w-5 h-5 shrink-0 ${isActive ? "text-accent" : "group-hover:text-foreground"}`}
                />
                {!collapsed && (
                  <span className="font-bold text-sm uppercase tracking-widest">
                    {t(item.key)}
                  </span>
                )}
                {isActive && !collapsed && (
                  <div className="ml-auto w-1 h-4 rounded-full bg-accent shadow-[0_0_8px_rgba(99,102,241,0.6)]" aria-hidden="true" />
                )}
              </Link>
            );
          })}
        </nav>

        {/* Footer / Settings Section */}
        <div className="p-4 border-t border-border space-y-2">
          {!collapsed && (
            <div className="px-4 py-2 mb-2" role="status" aria-live="polite">
              <div className="flex items-center gap-2 mb-1">
                <div className="w-2 h-2 rounded-full bg-green-500 grow-success" aria-hidden="true" />
                <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-tighter">
                  {t("systemNominal")}
                </span>
              </div>
              <div className="text-[10px] font-mono text-muted-foreground/50 tabular-nums uppercase tracking-tighter">
                RPC_ID: STLR_MAIN_01
              </div>
            </div>
          )}

          {!collapsed && (
            <div className="px-2 py-1">
              <LanguageSwitcher />
            </div>
          )}

          <button
            onClick={() => setCollapsed(!collapsed)}
            aria-label={collapsed ? t("expandSidebar") : t("collapseSidebar")}
            aria-expanded={!collapsed}
            className="w-full flex items-center gap-4 px-4 py-3 rounded-xl text-muted-foreground hover:bg-white/5 hover:text-foreground transition-all duration-300"
          >
            {collapsed ? (
              <ChevronRight className="w-5 h-5 shrink-0" aria-hidden="true" />
            ) : (
              <ChevronLeft className="w-5 h-5 shrink-0" aria-hidden="true" />
            )}
            {!collapsed && (
              <span className="text-xs font-bold uppercase tracking-widest">
                {t("collapse")}
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
                {t("settings")}
              </span>
            )}
          </Link>
        </div>
      </div>
    </aside>
  );
}
