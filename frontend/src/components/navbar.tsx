"use client";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Info, Phone, BookOpen, X, Menu } from "lucide-react";
import { ThemeToggle } from "./ThemeToggle";

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
    const timer = setTimeout(() => setMobileOpen(false), 0);
    return () => clearTimeout(timer);
  }, [pathname]);

  return (
    <>
      <nav
        className={`fixed top-0 right-0 left-0 z-40 transition-all duration-300 ${scrolled
            ? "navbar-scrolled"
            : "navbar-default"
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

          {/* Right side: Theme toggle + hamburger (mobile) + brand tag (desktop) */}
          <div className="flex items-center gap-2">
            <ThemeToggle />

            <button
              className="md:hidden navbar-hamburger"
              onClick={() => setMobileOpen((v) => !v)}
              aria-label="Toggle navigation menu"
              aria-expanded={mobileOpen}
            >
              {mobileOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
            </button>

            <div className="hidden md:flex items-center gap-2 navbar-brand-tag">
              <span className="w-1.5 h-1.5 rounded-full bg-accent" />
              <span>Stellar Insights</span>
            </div>
          </div>
        </div>


        <div className={`navbar-mobile-panel md:hidden ${mobileOpen ? "open" : ""}`}>
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
                  <div className="text-xs text-muted-foreground">{link.description}</div>
                </div>
                {isActive && <span className="ml-auto w-1.5 h-1.5 rounded-full bg-accent" />}
              </Link>
            );
          })}
        </div>
      </nav>


      <div className="h-14" />
    </>
  );
}

