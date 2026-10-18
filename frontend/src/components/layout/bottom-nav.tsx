"use client";

import React from "react";
import { Link } from "@/i18n/navigation";
import { usePathname } from "@/i18n/navigation";
import { Home, TrendingUp, Anchor, BarChart3, Bookmark } from "lucide-react";

interface NavItem {
  name: string;
  href: string;
  icon: React.ReactNode;
  id: string;
}

const navItems: NavItem[] = [
  {
    name: "Home",
    href: "/",
    icon: <Home className="w-5 h-5" />,
    id: "home",
  },
  {
    name: "Corridors",
    href: "/corridors",
    icon: <TrendingUp className="w-5 h-5" />,
    id: "corridors",
  },
  {
    name: "Anchors",
    href: "/anchors",
    icon: <Anchor className="w-5 h-5" />,
    id: "anchors",
  },
  {
    name: "Analytics",
    href: "/analytics",
    icon: <BarChart3 className="w-5 h-5" />,
    id: "analytics",
  },
  {
    name: "Saved",
    href: "/dashboard",
    icon: <Bookmark className="w-5 h-5" />,
    id: "saved",
  },
];

export function BottomNav() {
  const pathname = usePathname();

  const isActive = (href: string) => {
    if (href === "/") return pathname === href;
    return pathname === href || pathname.startsWith(href + "/");
  };

  return (
    <nav
      className="fixed bottom-0 left-0 right-0 glass border-t border-border md:hidden z-50"
      aria-label="Mobile bottom navigation"
      style={{ paddingBottom: "env(safe-area-inset-bottom, 0px)" }}
    >
      <div className="flex items-center justify-around h-16 px-1">
        {navItems.map((item) => {
          const active = isActive(item.href);
          return (
            <Link
              key={item.id}
              href={item.href}
              className={`flex flex-col items-center justify-center gap-0.5 flex-1 py-2 rounded-lg transition-colors min-h-[44px] touch-manipulation ${
                active
                  ? "text-accent"
                  : "text-muted-foreground active:bg-white/5"
              }`}
              aria-current={active ? "page" : undefined}
              aria-label={item.name}
            >
              <div className={`transition-transform ${active ? "scale-110" : ""}`}>
                {item.icon}
              </div>
              <span className={`text-[9px] font-mono uppercase tracking-wider ${active ? "text-accent" : ""}`}>
                {item.name}
              </span>
              {active && (
                <span className="absolute bottom-0 w-6 h-0.5 rounded-full bg-accent" aria-hidden="true" />
              )}
            </Link>
          );
        })}
      </div>
    </nav>
  );
}
