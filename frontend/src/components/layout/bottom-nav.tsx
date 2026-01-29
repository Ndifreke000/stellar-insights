"use client";

import React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Home, TrendingUp, Anchor, BarChart3 } from "lucide-react";

interface NavItem {
  name: string;
  href: string;
  icon: React.ReactNode;
  id: string;
}

const navItems: NavItem[] = [
  {
    name: "Dashboard",
    href: "/dashboard",
    icon: <Home className="w-5 h-5" />,
    id: "dashboard",
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
];

export function BottomNav() {
  const pathname = usePathname();

  const isActive = (href: string) => {
    return pathname === href || pathname.startsWith(href + "/");
  };

  return (
    <nav className="fixed bottom-0 left-0 right-0 bg-white dark:bg-slate-900 border-t border-gray-200 dark:border-slate-700 lg:hidden z-50 safe-area-inset-bottom">
      <div className="flex items-center justify-around h-16 px-2">
        {navItems.map((item) => {
          const active = isActive(item.href);
          return (
            <Link
              key={item.id}
              href={item.href}
              className={`flex flex-col items-center justify-center gap-1 px-3 py-2 rounded-lg transition-colors min-w-[44px] min-h-[44px] touch-manipulation ${
                active
                  ? "text-blue-500"
                  : "text-gray-600 dark:text-gray-400 active:bg-gray-100 dark:active:bg-slate-800"
              }`}
              aria-current={active ? "page" : undefined}
              aria-label={item.name}
            >
              <div className={active ? "scale-110" : ""}>{item.icon}</div>
              <span className="text-xs font-medium">{item.name}</span>
            </Link>
          );
        })}
      </div>
    </nav>
  );
}
