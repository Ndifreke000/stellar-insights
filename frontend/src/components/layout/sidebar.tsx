"use client";

import React, { useEffect, useRef } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Home, TrendingUp, Anchor, BarChart3, X } from "lucide-react";

interface SidebarProps {
  open: boolean;
  onClose: () => void;
}

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

export function Sidebar({ open, onClose }: SidebarProps) {
  const pathname = usePathname();
  const sidebarRef = useRef<HTMLDivElement>(null);
  const touchStartX = useRef<number>(0);
  const touchEndX = useRef<number>(0);

  const isActive = (href: string) => {
    return pathname === href || pathname.startsWith(href + "/");
  };

  // Swipe gesture handling
  useEffect(() => {
    const handleTouchStart = (e: TouchEvent) => {
      touchStartX.current = e.touches[0].clientX;
    };

    const handleTouchMove = (e: TouchEvent) => {
      touchEndX.current = e.touches[0].clientX;
    };

    const handleTouchEnd = () => {
      const swipeDistance = touchStartX.current - touchEndX.current;
      const minSwipeDistance = 50;

      // Swipe left to close
      if (swipeDistance > minSwipeDistance && open) {
        onClose();
      }
    };

    const sidebar = sidebarRef.current;
    if (sidebar && open) {
      sidebar.addEventListener("touchstart", handleTouchStart);
      sidebar.addEventListener("touchmove", handleTouchMove);
      sidebar.addEventListener("touchend", handleTouchEnd);

      return () => {
        sidebar.removeEventListener("touchstart", handleTouchStart);
        sidebar.removeEventListener("touchmove", handleTouchMove);
        sidebar.removeEventListener("touchend", handleTouchEnd);
      };
    }
  }, [open, onClose]);

  // Prevent body scroll when sidebar is open on mobile
  useEffect(() => {
    if (open) {
      document.body.style.overflow = "hidden";
    } else {
      document.body.style.overflow = "";
    }
    return () => {
      document.body.style.overflow = "";
    };
  }, [open]);

  return (
    <>
      {/* Mobile Overlay */}
      {open && (
        <div
          className="fixed inset-0 bg-black/50 z-30 lg:hidden"
          onClick={onClose}
          aria-hidden="true"
        />
      )}

      {/* Sidebar */}
      <aside
        ref={sidebarRef}
        className={`fixed left-0 top-16 h-[calc(100vh-64px)] w-64 bg-white dark:bg-slate-900 border-r border-gray-200 dark:border-slate-700 transition-transform duration-300 z-40 lg:translate-x-0 overflow-y-auto touch-pan-y ${
          open ? "translate-x-0" : "-translate-x-full"
        }`}
      >
        <div className="p-4 lg:hidden">
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-100 dark:hover:bg-slate-800 rounded-lg min-w-[44px] min-h-[44px] touch-manipulation active:bg-gray-200 dark:active:bg-slate-700"
            aria-label="Close sidebar"
          >
            <X className="w-6 h-6" />
          </button>
        </div>

        <nav className="flex flex-col gap-2 p-4">
          {navItems.map((item) => {
            const active = isActive(item.href);
            return (
              <Link
                key={item.id}
                href={item.href}
                onClick={onClose}
                className={`flex items-center gap-3 px-4 py-3 rounded-lg font-medium transition-colors min-h-[44px] touch-manipulation ${
                  active
                    ? "bg-blue-500 text-white"
                    : "text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-slate-800 active:bg-gray-200 dark:active:bg-slate-700"
                }`}
                aria-current={active ? "page" : undefined}
              >
                {item.icon}
                <span>{item.name}</span>
              </Link>
            );
          })}
        </nav>

        {/* Footer Info */}
        <div className="absolute bottom-0 left-0 right-0 border-t border-gray-200 dark:border-slate-700 bg-gray-50 dark:bg-slate-800 p-4">
          <div className="text-xs text-gray-600 dark:text-gray-400">
            <p className="font-medium mb-1">Stellar Insights</p>
            <p>Payment Network Intelligence</p>
          </div>
        </div>
      </aside>

      {/* Desktop Sidebar - Always Visible */}
      <aside className="hidden lg:block fixed left-0 top-16 h-[calc(100vh-64px)] w-64 bg-white dark:bg-slate-900 border-r border-gray-200 dark:border-slate-700 overflow-y-auto">
        <nav className="flex flex-col gap-2 p-4">
          {navItems.map((item) => {
            const active = isActive(item.href);
            return (
              <Link
                key={item.id}
                href={item.href}
                className={`flex items-center gap-3 px-4 py-3 rounded-lg font-medium transition-colors ${
                  active
                    ? "bg-blue-500 text-white"
                    : "text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-slate-800"
                }`}
                aria-current={active ? "page" : undefined}
              >
                {item.icon}
                <span>{item.name}</span>
              </Link>
            );
          })}
        </nav>

        {/* Footer Info */}
        <div className="absolute bottom-0 left-0 right-0 border-t border-gray-200 dark:border-slate-700 bg-gray-50 dark:bg-slate-800 p-4">
          <div className="text-xs text-gray-600 dark:text-gray-400">
            <p className="font-medium mb-1">Stellar Insights</p>
            <p>Payment Network Intelligence</p>
          </div>
        </div>
      </aside>
    </>
  );
}
