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
  ChevronDown,
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
  Gauge,
  X,
} from "lucide-react";
import { useUserPreferences } from "@/contexts/UserPreferencesContext";
import { LanguageSwitcher } from "@/components/LanguageSwitcher";
import { BookmarksSidebarSection } from "@/components/BookmarksSidebarSection";

// Pinned items always show at the top, ungrouped.
const pinnedItems = [
  { key: "home", icon: LayoutDashboard, path: "/" },
  { key: "terminal", icon: LayoutDashboard, path: "/dashboard" },
];

// Everything else is grouped into collapsible sections so the sidebar
// doesn't read as one long undifferentiated list of 16+ links.
const navGroups = [
  {
    key: "networkData",
    items: [
      { key: "corridors", icon: Compass, path: "/corridors" },
      { key: "network", icon: Share2, path: "/network" },
      { key: "trustlines", icon: Users, path: "/trustlines" },
      { key: "liquidity", icon: Waves, path: "/liquidity" },
      { key: "pools", icon: Droplets, path: "/liquidity-pools" },
    ],
  },
  {
    key: "analytics",
    items: [
      { key: "analytics", icon: BarChart3, path: "/analytics" },
      { key: "apiUsage", icon: Activity, path: "/analytics/api" },
      { key: "calculator", icon: Calculator, path: "/calculator" },
      { key: "performance", icon: Gauge, path: "/performance" },
      { key: "chartExport", icon: BarChart3, path: "/analytics/charts" },
    ],
  },
  {
    key: "monitoring",
    items: [
      { key: "networkHealth", icon: Activity, path: "/health" },
      { key: "alerts", icon: Activity, path: "/alerts" },
      { key: "forecasting", icon: TrendingUp, path: "/corridors/forecasting" },
    ],
  },
  {
    key: "developer",
    items: [
      { key: "apiKeys", icon: Key, path: "/developer/keys" },
      { key: "governance", icon: ScrollText, path: "/governance" },
      { key: "quests", icon: Trophy, path: "/quests" },
      { key: "privacy", icon: Shield, path: "/settings/gdpr" },
      { key: "sep6", icon: Database, path: "/sep6" },
    ],
  },
];

interface SidebarProps {
  open?: boolean;
  onClose?: () => void;
}

type NavItem = { key: string; icon: typeof LayoutDashboard; path: string };

function NavLink({
  item,
  isActive,
  collapsed,
  label,
  onClick,
}: {
  item: NavItem;
  isActive: boolean;
  collapsed: boolean;
  label: string;
  onClick?: () => void;
}) {
  const Icon = item.icon;
  return (
    <Link
      href={item.path}
      onClick={onClick}
      aria-current={isActive ? "page" : undefined}
      aria-label={label}
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
          {label}
        </span>
      )}
      {isActive && !collapsed && (
        <div className="ml-auto w-1 h-4 rounded-full bg-accent shadow-[0_0_8px_rgba(99,102,241,0.6)]" aria-hidden="true" />
      )}
    </Link>
  );
}

export function Sidebar({ open = false, onClose }: SidebarProps = {}) {
  const pathname = usePathname();
  const t = useTranslations("layout.sidebar");
  const tGroups = useTranslations("layout.sidebar.groups");
  const { prefs, setPrefs } = useUserPreferences();
  const collapsed = prefs.sidebarCollapsed;
  const setCollapsed = (val: boolean) => setPrefs({ sidebarCollapsed: val });

  const collapsedGroups = prefs.sidebarCollapsedGroups;
  const toggleGroup = (groupKey: string) => {
    setPrefs({
      sidebarCollapsedGroups: collapsedGroups.includes(groupKey)
        ? collapsedGroups.filter((k) => k !== groupKey)
        : [...collapsedGroups, groupKey],
    });
  };

  const sidebarContent = (
    <div className="flex flex-col h-full">
      {/* Logo Section */}
      <div className="p-6 flex items-center gap-3">
        <div className="w-8 h-8 bg-accent rounded-lg flex items-center justify-center glow-accent shrink-0" aria-hidden="true">
          <TrendingUp className="w-5 h-5 text-white" aria-hidden="true" />
        </div>
        {!collapsed && (
          <span className="text-xl font-bold tracking-tighter text-foreground whitespace-nowrap overflow-hidden">
            PAY
            <span className="text-accent underline decoration-accent/30">
              RAIDER
            </span>
          </span>
        )}
        {/* Mobile close button */}
        {onClose && (
          <button
            onClick={onClose}
            aria-label="Close sidebar"
            className="md:hidden ml-auto p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-white/5 transition-colors"
          >
            <X className="w-4 h-4" aria-hidden="true" />
          </button>
        )}
      </div>

      {/* Navigation Section */}
      <nav aria-label="Primary navigation" className="flex-1 px-4 py-8 overflow-y-auto">
        <ul role="list" className="space-y-3 m-0 p-0 list-none">
          {pinnedItems.map((item) => (
            <li key={item.path}>
              <NavLink
                item={item}
                isActive={pathname === item.path}
                collapsed={collapsed}
                label={t(item.key)}
                onClick={onClose}
              />
            </li>
          ))}
        </ul>

        <div className="mt-6 space-y-4">
          {navGroups.map((group) => {
            const hasActiveItem = group.items.some((item) => pathname === item.path);
            const expanded = hasActiveItem || !collapsedGroups.includes(group.key);
            const panelId = `sidebar-group-${group.key}`;

            return (
              <div key={group.key}>
                {!collapsed && (
                  <button
                    onClick={() => toggleGroup(group.key)}
                    aria-expanded={expanded}
                    aria-controls={panelId}
                    className="w-full flex items-center justify-between px-4 py-1.5 text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70 hover:text-foreground transition-colors"
                  >
                    <span>{tGroups(group.key)}</span>
                    <ChevronDown
                      aria-hidden="true"
                      className={`w-3.5 h-3.5 shrink-0 transition-transform duration-200 ${expanded ? "" : "-rotate-90"}`}
                    />
                  </button>
                )}
                {(collapsed || expanded) && (
                  <ul id={panelId} role="list" className="space-y-3 m-0 p-0 list-none mt-1">
                    {group.items.map((item) => (
                      <li key={item.path}>
                        <NavLink
                          item={item}
                          isActive={pathname === item.path}
                          collapsed={collapsed}
                          label={t(item.key)}
                          onClick={onClose}
                        />
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            );
          })}
        </div>
      </nav>

      {/* Bookmarks Section */}
      <div className="px-4 border-t border-border/50 pt-3">
        <BookmarksSidebarSection collapsed={collapsed} />
      </div>

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

        {/* Only show collapse toggle on desktop */}
        <button
          onClick={() => setCollapsed(!collapsed)}
          aria-label={collapsed ? t("expandSidebar") : t("collapseSidebar")}
          aria-expanded={!collapsed}
          className="hidden md:flex w-full items-center gap-4 px-4 py-3 rounded-xl text-muted-foreground hover:bg-white/5 hover:text-foreground transition-all duration-300"
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
          aria-label="Navigate to Settings"
          className="flex items-center gap-4 px-4 py-3 rounded-xl text-muted-foreground hover:bg-white/5 hover:text-foreground transition-all duration-300"
          onClick={onClose}
        >
          <Settings className="w-5 h-5 shrink-0" aria-hidden="true" />
          {!collapsed && (
            <span className="text-xs font-bold uppercase tracking-widest">
              {t("settings")}
            </span>
          )}
        </Link>
      </div>
    </div>
  );

  return (
    <>
      {/* Desktop sidebar — always visible on md+ */}
      <aside
        aria-label="Sidebar navigation"
        className={`hidden md:block fixed top-0 left-0 h-screen overflow-y-auto glass border-r border-border transition-all duration-500 z-50 ${collapsed ? "w-20" : "w-64"
          }`}
      >
        {sidebarContent}
      </aside>

      {/* Mobile sidebar — drawer overlay */}
      {open && (
        <>
          {/* Backdrop */}
          <div
            className="md:hidden fixed inset-0 bg-black/60 backdrop-blur-sm z-40"
            onClick={onClose}
            aria-hidden="true"
          />
          {/* Drawer */}
          <aside
            aria-label="Sidebar navigation"
            className="md:hidden fixed top-0 left-0 h-screen w-72 overflow-y-auto glass border-r border-border z-50 animate-in slide-in-from-left duration-300"
          >
            {sidebarContent}
          </aside>
        </>
      )}
    </>
  );
}