"use client";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  Info,
  Phone,
  BookOpen,
  X,
  Menu,
  LayoutDashboard,
  Compass,
  BarChart3,
  Activity,
  Waves,
  Droplets,
  Link2,
} from "lucide-react";

const sidebarNavItems = [
  { name: "Home", icon: LayoutDashboard, path: "/" },
  { name: "Terminal", icon: LayoutDashboard, path: "/dashboard" },
  { name: "Corridors", icon: Compass, path: "/corridors" },
  { name: "Analytics", icon: BarChart3, path: "/analytics" },
  { name: "Network Health", icon: Activity, path: "/health" },
  { name: "Liquidity", icon: Waves, path: "/liquidity" },
  { name: "Pools", icon: Droplets, path: "/liquidity-pools" },
  { name: "Trustlines", icon: Link2, path: "/trustlines" },
];

const navLinks = [
  {
    name: "About Us",
    href: "/about",
    icon: Info,
    description: "Learn about Stellar Insights",
  },
  {
    name: "How to Use",
    href: "/how-to-use",
    icon: BookOpen,
    description: "Get started with our platform",
  },
  {
    name: "Contact Us",
    href: "/contact",
    icon: Phone,
    description: "Reach out to our team",
  },
];

export function Navbar() {
  const pathname = usePathname();
  const [scrolled, setScrolled] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);

  useEffect(() => {
    const handleScroll = () => setScrolled(window.scrollY > 10);
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  useEffect(() => {
    setMobileOpen(false);
  }, [pathname]);

  return (
    <>
      <nav
        className={`fixed top-0 right-0 left-0 z-40 transition-all duration-300 ${
          scrolled ? "navbar-scrolled" : "navbar-default"
        }`}
        style={{ paddingLeft: "var(--sidebar-offset, 5rem)" }}
      >
        <div className="navbar-inner flex items-center justify-between h-14 px-6">
          <div className="flex items-center gap-2">
            <span className="navbar-live-dot" />
            <span className="navbar-live-text">LIVE NETWORK</span>
          </div>

          <ul className="navbar-links hidden md:flex items-center gap-1">
            {navLinks.map((link) => {
              const isActive = pathname === link.href;
              const Icon = link.icon;
              return (
                <li key={link.href}>
                  <Link
                    href={link.href}
                    className={`navbar-link ${isActive ? "navbar-link--active" : ""}`}
                  >
                    <Icon className="w-4 h-4 shrink-0" />
                    <span>{link.name}</span>
                    {isActive && <span className="navbar-link-indicator" />}
                  </Link>
                </li>
              );
            })}
          </ul>

          <button
            className="md:hidden navbar-hamburger"
            onClick={() => setMobileOpen((v) => !v)}
            aria-label="Toggle navigation menu"
            aria-expanded={mobileOpen}
          >
            {mobileOpen ? (
              <X className="w-5 h-5" />
            ) : (
              <Menu className="w-5 h-5" />
            )}
          </button>

          <div className="hidden md:flex items-center gap-2 navbar-brand-tag">
            <span className="w-1.5 h-1.5 rounded-full bg-accent" />
            <span>Stellar Insights</span>
          </div>
        </div>

        <div
          className={`navbar-mobile-panel lg:hidden ${mobileOpen ? "open" : ""}`}
        >
          {/* Main app navigation (from sidebar) */}
          <div className="px-2 pt-2 pb-1">
            <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] px-3 mb-2">
              Navigation
            </div>
            <div className="grid grid-cols-2 gap-1">
              {sidebarNavItems.map((item) => {
                const isActive = pathname === item.path;
                const Icon = item.icon;
                return (
                  <Link
                    key={item.path}
                    href={item.path}
                    className={`flex items-center gap-2.5 px-3 py-2.5 rounded-lg transition-all duration-200 ${
                      isActive
                        ? "bg-accent/10 text-accent border border-accent/20"
                        : "text-muted-foreground hover:bg-white/5 hover:text-foreground border border-transparent"
                    }`}
                  >
                    <Icon
                      className={`w-4 h-4 shrink-0 ${isActive ? "text-accent" : ""}`}
                    />
                    <span className="text-xs font-bold uppercase tracking-wider">
                      {item.name}
                    </span>
                  </Link>
                );
              })}
            </div>
          </div>

          {/* Divider */}
          <div className="border-t border-border/30 mx-4 my-2" />

          {/* Info links */}
          <div className="px-2 pb-2">
            <div className="text-[10px] font-mono text-muted-foreground/50 uppercase tracking-[0.2em] px-3 mb-2">
              Info
            </div>
            {navLinks.map((link) => {
              const isActive = pathname === link.href;
              const Icon = link.icon;
              return (
                <Link
                  key={link.href}
                  href={link.href}
                  className={`navbar-mobile-link ${isActive ? "navbar-mobile-link--active" : ""}`}
                >
                  <div className="navbar-mobile-icon-wrap">
                    <Icon className="w-4 h-4" />
                  </div>
                  <div>
                    <div className="font-semibold text-sm">{link.name}</div>
                    <div className="text-xs text-muted-foreground">
                      {link.description}
                    </div>
                  </div>
                  {isActive && (
                    <span className="ml-auto w-1.5 h-1.5 rounded-full bg-accent" />
                  )}
                </Link>
              );
            })}
          </div>
        </div>
      </nav>

      <div className="h-14" />
    </>
  );
}
