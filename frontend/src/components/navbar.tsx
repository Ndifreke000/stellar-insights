"use client";

import React, { useState, useEffect, useRef } from "react";
import { Link, usePathname } from "@/i18n/navigation";
import { useTranslations } from "next-intl";
import { Info, Phone, BookOpen, X, Menu } from "lucide-react";
import { ThemeToggle } from "./ThemeToggle";
import { NotificationBell } from "./notifications";

const navLinkKeys = [
  { key: "aboutUs", href: "/about", icon: Info, descKey: "learnAbout" },
  { key: "howToUse", href: "/how-to-use", icon: BookOpen, descKey: "getStarted" },
  { key: "contactUs", href: "/contact", icon: Phone, descKey: "reachOut" },
];

export function Navbar() {
  const pathname = usePathname();
  const t = useTranslations("navbar");
  const [scrolled, setScrolled] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const mobilePanelRef = useRef<HTMLDivElement | null>(null);
  const firstMobileLinkRef = useRef<HTMLAnchorElement | null>(null);

  useEffect(() => {
    const handleScroll = () => setScrolled(window.scrollY > 10);
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => setMobileOpen(false), 0);
    return () => clearTimeout(timer);
  }, [pathname]);

  // Lock body scroll when mobile nav is open
  useEffect(() => {
    if (mobileOpen) {
      document.body.style.overflow = "hidden";
      // focus the first mobile link for keyboard users
      setTimeout(() => firstMobileLinkRef.current?.focus(), 0);
    } else {
      document.body.style.overflow = "";
    }

    return () => {
      document.body.style.overflow = "";
    };
  }, [mobileOpen]);

  // Close on Escape and click-outside
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setMobileOpen(false);
    }

    function onClick(e: MouseEvent) {
      if (!mobileOpen) return;
      const panel = mobilePanelRef.current;
      const target = e.target as Node | null;
      if (
        panel &&
        target &&
        !panel.contains(target) &&
        !(target as Element).closest(".navbar-hamburger")
      ) {
        setMobileOpen(false);
      }
    }

    document.addEventListener("keydown", onKey);
    document.addEventListener("click", onClick);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("click", onClick);
    };
  }, [mobileOpen]);

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
            <span className="navbar-live-text">{t("liveNetwork")}</span>
          </div>

          <ul className="navbar-links hidden md:flex items-center gap-1">
            {navLinkKeys.map((link) => {
              const isActive = pathname === link.href;
              const Icon = link.icon;
              return (
                <li key={link.href}>
                  <Link
                    href={link.href}
                    className={`navbar-link ${isActive ? "navbar-link--active" : ""}`}
                  >
                    <Icon className="w-4 h-4 shrink-0" />
                    <span>{t(link.key)}</span>
                    {isActive && <span className="navbar-link-indicator" />}
                  </Link>
                </li>
              );
            })}
          </ul>

          {/* Right side: Theme toggle + notifications + hamburger (mobile) + brand tag (desktop) */}
          <div className="flex items-center gap-2">
            <ThemeToggle />
            <NotificationBell />

            <button
              className="md:hidden navbar-hamburger"
              onClick={() => setMobileOpen((v) => !v)}
              aria-label={t("toggleMenu")}
              aria-expanded={mobileOpen}
              aria-controls="mobile-nav-panel"
            >
              {mobileOpen ? (
                <X className="w-5 h-5" />
              ) : (
                <Menu className="w-5 h-5" />
              )}
            </button>

            <div className="hidden md:flex items-center gap-2 navbar-brand-tag">
              <span className="w-1.5 h-1.5 rounded-full bg-accent" />
              <span>{t("stellarInsights")}</span>
            </div>
          </div>
        </div>

        <div
          id="mobile-nav-panel"
          ref={mobilePanelRef}
          className={`navbar-mobile-panel md:hidden ${mobileOpen ? "open" : ""}`}
          role="menu"
          aria-hidden={!mobileOpen}
        >
          {navLinkKeys.map((link) => {
            const isActive = pathname === link.href;
            const Icon = link.icon;
            return (
              <Link
                key={link.href}
                href={link.href}
                className={`navbar-mobile-link ${isActive ? "navbar-mobile-link--active" : ""}`}
                ref={firstMobileLinkRef}
                role="menuitem"
                tabIndex={mobileOpen ? 0 : -1}
              >
                <div className="navbar-mobile-icon-wrap">
                  <Icon className="w-4 h-4" />
                </div>
                <div>
                  <div className="font-semibold text-sm">{t(link.key)}</div>
                  <div className="text-xs text-muted-foreground">{t(link.descKey)}</div>
                </div>
                {isActive && (
                  <span className="ml-auto w-1.5 h-1.5 rounded-full bg-accent" />
                )}
              </Link>
            );
          })}
        </div>
      </nav>

      <div className="h-14" />
    </>
  );
}
