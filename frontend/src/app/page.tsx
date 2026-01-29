"use client";

import Link from "next/link";
import {
  ArrowRight,
  TrendingUp,
  Zap,
  BarChart3,
  AlertCircle,
  CheckCircle2,
  Menu,
  X,
  Wallet,
} from "lucide-react";
import { useState, useEffect, useRef } from "react";
import { useWallet } from "../components/lib/wallet-context";
import { useNotifications } from "../contexts/NotificationContext";

export default function Home() {
  const {
    isConnected,
    address,
    isConnecting,
    connectWallet,
    disconnectWallet,
  } = useWallet();
  const { showToast } = useNotifications();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [showWalletMenu, setShowWalletMenu] = useState(false);
  const prevConnectedRef = useRef<boolean | null>(null);

  // Show notification when wallet connection status changes
  useEffect(() => {
    // Skip the first render to avoid showing notification on initial load
    if (prevConnectedRef.current === null) {
      prevConnectedRef.current = isConnected;
      return;
    }

    // Only show notification if the connection status actually changed
    if (prevConnectedRef.current !== isConnected) {
      const displayAddress = address
        ? `${address.slice(0, 6)}...${address.slice(-4)}`
        : null;

      if (isConnected && address) {
        showToast({
          type: 'success',
          priority: 'medium',
          title: 'Wallet Connected',
          message: `Successfully connected to ${displayAddress}`,
          category: 'system',
          duration: 4000,
        });
      } else if (!isConnected && prevConnectedRef.current) {
        showToast({
          type: 'info',
          priority: 'medium',
          title: 'Wallet Disconnected',
          message: 'Your wallet has been disconnected',
          category: 'system',
          duration: 3000,
        });
      }
      prevConnectedRef.current = isConnected;
    }
  }, [isConnected, address, showToast]);

  const handleConnect = async () => {
    try {
      await connectWallet();
    } catch (error) {
      showToast({
        type: 'error',
        priority: 'high',
        title: 'Connection Failed',
        message: 'Failed to connect wallet. Please try again.',
        category: 'system',
        duration: 5000,
      });
    }
  };

  const handleDisconnect = () => {
    try {
      disconnectWallet();
      setShowWalletMenu(false);
    } catch (error) {
      showToast({
        type: 'error',
        priority: 'medium',
        title: 'Disconnection Failed',
        message: 'Failed to disconnect wallet properly.',
        category: 'system',
        duration: 4000,
      });
    }
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      {/* Navigation */}
      <nav className="fixed w-full top-0 z-50 bg-background/80 backdrop-blur-sm border-b border-gray-500">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 bg-blue-500 rounded-lg flex items-center justify-center">
                <TrendingUp className="w-5 h-5 text-primary-foreground" />
              </div>
              <span className="text-xl font-bold">Stellar Insights</span>
            </div>

            {/* Desktop Navigation */}
            <div className="hidden md:flex items-center gap-8">
              <a
                href="#problem"
                className="text-sm font-medium hover:text-blue-500"
              >
                Problem
              </a>
              <a
                href="#solution"
                className="text-sm font-medium hover:text-blue-500"
              >
                Solution
              </a>
              <a
                href="#features"
                className="text-sm font-medium hover:text-blue-500"
              >
                Features
              </a>
              <Link
                href="/corridors/"
                className="text-sm font-medium hover:text-blue-500"
              >
                Corridors
              </Link>

              {isConnected && address ? (
                <div className="relative">
                  <button
                    onClick={() => setShowWalletMenu(!showWalletMenu)}
                    className="px-6 py-2 bg-blue-500 text-primary-foreground rounded-full font-medium hover:opacity-90 transition flex items-center gap-2"
                  >
                    <Wallet className="w-4 h-4" />
                    {address.slice(0, 6)}...{address.slice(-4)}
                  </button>

                  {showWalletMenu && (
                    <div className="absolute right-0 mt-2 w-48 bg-card border border-border rounded-lg shadow-lg py-2 z-50">
                      <div className="px-4 py-2 text-sm text-muted-foreground border-b border-border">
                        {address}
                      </div>
                      <button
                        onClick={() => {
                          handleDisconnect();
                          setShowWalletMenu(false);
                        }}
                        className="w-full text-left px-4 py-2 text-sm hover:bg-blue-900 transition"
                      >
                        Disconnect
                      </button>
                    </div>
                  )}
                </div>
              ) : (
                <button
                  onClick={handleConnect}
                  disabled={isConnecting}
                  className="px-6 py-2 bg-blue-500 text-primary-foreground rounded-full font-medium hover:opacity-90 transition disabled:opacity-50 flex items-center gap-2"
                >
                  <Wallet className="w-4 h-4" />
                  {isConnecting ? "Connecting..." : "Connect Wallet"}
                </button>
              )}
            </div>

            {/* Mobile Menu Button */}
            <button
              className="md:hidden p-2"
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
              aria-label="Toggle menu"
            >
              {mobileMenuOpen ? (
                <X className="w-6 h-6" />
              ) : (
                <Menu className="w-6 h-6" />
              )}
            </button>
          </div>

          {/* Mobile Navigation */}
          {mobileMenuOpen && (
            <div className="md:hidden pb-4 space-y-3">
              <a
                href="#problem"
                className="block text-sm font-medium hover:text-blue-500"
              >
                Problem
              </a>
              <a
                href="#solution"
                className="block text-sm font-medium hover:text-blue-500"
              >
                Solution
              </a>
              <a
                href="#features"
                className="block text-sm font-medium hover:text-blue-500"
              >
                Features
              </a>
              <Link
                href="/corridors/"
                className="block text-sm font-medium hover:text-blue-500"
              >
                Corridors
              </Link>

              {isConnected && address ? (
                <button
                  onClick={() => {
                    handleDisconnect();
                    setMobileMenuOpen(false);
                  }}
                  className="w-full px-6 py-2 bg-blue-500 text-secondary-foreground rounded-full font-medium hover:opacity-90 transition text-sm"
                >
                  Disconnect: {address.slice(0, 6)}...{address.slice(-4)}
                </button>
              ) : (
                <button
                  onClick={() => {
                    handleConnect();
                    setMobileMenuOpen(false);
                  }}
                  disabled={isConnecting}
                  className="w-full px-6 py-2 bg-blue-500 text-primary-foreground rounded-full font-medium hover:opacity-90 transition disabled:opacity-50 flex items-center justify-center gap-2"
                >
                  <Wallet className="w-4 h-4" />
                  {isConnecting ? "Connecting..." : "Connect Wallet"}
                </button>
              )}
            </div>
          )}
        </div>
      </nav>

      {/* Hero */}
      <section className="pt-32 pb-20 px-4 sm:px-6 lg:px-8 relative overflow-hidden">
        <div className="max-w-4xl mx-auto">
          <div className="absolute top-20 right-0 w-96 h-96 bg-primary/10 rounded-full blur-3xl -z-10" />
          <div className="absolute top-40 left-0 w-80 h-80 bg-accent/5 rounded-full blur-3xl -z-10" />

          <div className="space-y-8 text-center">
            <div className="inline-block px-4 py-2 bg-blue-500/10 border border-blue-500/30 rounded-full">
              <span className="text-primary font-medium text-sm">
                Stellar Payment Intelligence
              </span>
            </div>

            <h1 className="text-5xl sm:text-6xl font-bold tracking-tight">
              <span className="text-balance">
                Payment Reliability,
                <br />
                <span className="bg-linear-to-r from-blue-500 to-accent bg-clip-text text-transparent">
                  Not Just Speed
                </span>
              </span>
            </h1>

            <p className="text-lg sm:text-xl text-center max-w-2xl mx-auto flex justify-center">
              Deep insights into Stellar payment network performance. Predict
              success, optimize routing, and identify liquidity bottlenecks
              before they impact your business.
            </p>

            <div className="flex flex-col sm:flex-row gap-4 justify-center pt-4">
              {isConnected ? (
                <>
                  <Link
                    href="/dashboard"
                    className="px-8 py-4 bg-blue-500 text-primary-foreground rounded-full font-semibold hover:opacity-90 transition flex items-center justify-center gap-2"
                  >
                    Access Dashboard <ArrowRight className="w-5 h-5" />
                  </Link>
                  <button className="px-8 py-4 bg-blue-9000 text-secondary-foreground rounded-full font-semibold hover:opacity-90 transition">
                    View Documentation
                  </button>
                </>
              ) : (
                <>
                  <button
                    onClick={handleConnect}
                    disabled={isConnecting}
                    className="px-8 py-4 bg-blue-500 text-primary-foreground rounded-full font-semibold hover:opacity-90 transition disabled:opacity-50 flex items-center justify-center gap-2"
                  >
                    <Wallet className="w-5 h-5" />
                    {isConnecting ? "Connecting..." : "Connect Wallet to Start"}
                    {!isConnecting && <ArrowRight className="w-5 h-5" />}
                  </button>
                  <button className="px-8 py-4 bg-gray-400 text-secondary-foreground rounded-full font-semibold hover:opacity-90 transition">
                    Watch Demo
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* Problem Section */}
      <section
        id="problem"
        className="py-20 px-4 sm:px-6 lg:px-8 bg-gray-700/30"
      >
        <div className="max-w-6xl mx-auto">
          <div className="text-center mb-16">
            <h2 className="text-4xl sm:text-5xl font-bold mb-4">
              The Challenge
            </h2>
            <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
              Stellar is powerful for stablecoins, payments, and asset issuance,
              but speed alone isn&apos;t enough
            </p>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            {[
              {
                icon: <TrendingUp className="w-6 h-6" />,
                title: "Asset Corridor Reliability",
                description:
                  "Which payment corridors consistently succeed vs fail? Current tools don't answer this.",
              },
              {
                icon: <AlertCircle className="w-6 h-6" />,
                title: "Liquidity Uncertainty",
                description:
                  "Is there enough liquidity in major payment paths? Bottlenecks appear without warning.",
              },
              {
                icon: <BarChart3 className="w-6 h-6" />,
                title: "Anchor & Asset Bottlenecks",
                description:
                  "Certain anchors or assets block transfers. Performance data is hard to access.",
              },
              {
                icon: <Zap className="w-6 h-6" />,
                title: "Market Efficiency Unknown",
                description:
                  "Are markets efficient, stable, or unreliable under stress? Raw transactions don't tell the story.",
              },
            ].map((item, i) => (
              <div
                key={i}
                className="p-6 rounded-xl bg-card border border-gray-500 hover:border-blue-500/50 transition group"
              >
                <div className="w-12 h-12 bg-blue-500/10 rounded-lg flex items-center justify-center text-blue-500 mb-4 group-hover:bg-primary/20 transition">
                  {item.icon}
                </div>
                <h3 className="text-lg font-semibold mb-2">{item.title}</h3>
                <p className="text-muted-foreground">{item.description}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Solution Section */}
      <section id="solution" className="py-20 px-4 sm:px-6 lg:px-8">
        <div className="max-w-6xl mx-auto">
          <div className="text-center mb-16">
            <h2 className="text-4xl sm:text-5xl font-bold mb-4">
              The Solution
            </h2>
            <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
              Stellar Insights Intelligence Dashboard for real-world payment
              network visibility
            </p>
          </div>

          <div className="space-y-12">
            {[
              {
                title: "Predict Payment Success",
                description:
                  "Know the likelihood of payment success before sending. Make intelligent routing decisions based on real-time network data.",
                icon: <CheckCircle2 className="w-8 h-8" />,
                color: "from-primary",
              },
              {
                title: "Optimize Routing Paths",
                description:
                  "Discover the most reliable paths through the network. Avoid bottlenecks and ensure payments reach their destination.",
                icon: <TrendingUp className="w-8 h-8" />,
                color: "from-accent",
              },
              {
                title: "Quantify Real-World Reliability",
                description:
                  "Move beyond TPS metrics. Understand actual payment success rates, latency, and failure patterns across corridors.",
                icon: <BarChart3 className="w-8 h-8" />,
                color: "from-primary",
              },
              {
                title: "Identify Liquidity Bottlenecks",
                description:
                  "See where liquidity is constrained. Improve provisioning strategies with data-driven insights into market gaps.",
                icon: <Zap className="w-8 h-8" />,
                color: "from-accent",
              },
              {
                title: "Understand Ecosystem Health",
                description:
                  "Track network risk and health trends. Anticipate problems before they impact payments and make proactive decisions.",
                icon: <AlertCircle className="w-8 h-8" />,
                color: "from-primary",
              },
            ].map((item, i) => (
              <div
                key={i}
                className="flex flex-col md:flex-row gap-8 items-center"
              >
                <div
                  className={`shrink-0 w-16 h-16 bg-linear-to-br ${item.color} to-blue-500/50 rounded-xl flex items-center justify-center text-primary-foreground`}
                >
                  {item.icon}
                </div>
                <div className="flex-1">
                  <h3 className="text-2xl font-bold mb-2">{item.title}</h3>
                  <p className="text-lg text-muted-foreground">
                    {item.description}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features */}
      <section
        id="features"
        className="py-20 px-4 sm:px-6 lg:px-8 bg-gray-700/30"
      >
        <div className="max-w-6xl mx-auto">
          <div className="text-center mb-16">
            <h2 className="text-4xl sm:text-5xl font-bold mb-4">
              Built for Your Role
            </h2>
            <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
              Whether you run a wallet, build on Stellar, operate an anchor, or
              manage a business, we have insights for you
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
            {[
              {
                role: "Wallets",
                benefit: "Show users payment reliability before confirmation",
              },
              {
                role: "Developers",
                benefit:
                  "Build smarter payment logic with network intelligence",
              },
              {
                role: "Businesses",
                benefit: "Ensure payment success in cross-border operations",
              },
              {
                role: "Anchors",
                benefit: "Monitor liquidity provisioning and optimize reserves",
              },
            ].map((item, i) => (
              <div
                key={i}
                className="p-6 rounded-lg bg-card border border-gray-500 hover:border-blue-500/50 transition cursor-pointer"
              >
                <h3 className="font-semibold text-lg mb-2">{item.role}</h3>
                <p className="text-sm text-muted-foreground">{item.benefit}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-20 px-4 sm:px-6 lg:px-8">
        <div className="max-w-3xl mx-auto">
          <div className="bg-linear-to-r from-blue-500/20 to-accent/20 border border-blue-500/30 rounded-2xl p-12 text-center">
            <h2 className="text-3xl sm:text-4xl font-bold mb-4">
              {isConnected
                ? "Welcome to Stellar Insights"
                : "Ready to See the Whole Picture?"}
            </h2>
            <p className="text-lg text-muted-foreground mb-8">
              {isConnected
                ? `Connected with ${address?.slice(0, 6)}...${address?.slice(-4)}. Explore deep insights into Stellar payment network performance.`
                : "Connect your Stellar wallet to unlock deep insights into payment network performance and liquidity health."}
            </p>

            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              {isConnected ? (
                <>
                  <button className="px-8 py-3 bg-blue-500 text-primary-foreground rounded-full font-semibold hover:opacity-90 transition">
                    Go to Dashboard
                  </button>
                  <button className="px-8 py-3 bg-gray-500 text-secondary-foreground rounded-full font-semibold hover:opacity-90 transition">
                    View API Docs
                  </button>
                </>
              ) : (
                <>
                  <button
                    onClick={handleConnect}
                    disabled={isConnecting}
                    className="px-8 py-3 bg-blue-500 text-primary-foreground rounded-full font-semibold hover:opacity-90 transition disabled:opacity-50 flex items-center justify-center gap-2"
                  >
                    <Wallet className="w-4 h-4" />
                    {isConnecting ? "Connecting..." : "Connect Wallet"}
                  </button>
                  <button className="px-8 py-3 bg-gray-700 text-secondary-foreground rounded-full font-semibold hover:opacity-90 transition">
                    Schedule a Demo
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="py-12 px-4 sm:px-6 lg:px-8 border-t border-border">
        <div className="max-w-6xl mx-auto">
          <div className="grid md:grid-cols-4 gap-8 mb-8">
            <div>
              <div className="flex items-center gap-2 mb-4">
                <div className="w-6 h-6 bg-blue-500 rounded-lg flex items-center justify-center">
                  <TrendingUp className="w-4 h-4 text-primary-foreground" />
                </div>
                <span className="font-bold">Stellar Insights</span>
              </div>
              <p className="text-sm text-muted-foreground">
                Payment network intelligence for Stellar.
              </p>
            </div>

            <div>
              <h4 className="font-semibold mb-4 text-gray-500">Product</h4>
              <ul className="space-y-2 text-sm">
                <li>
                  <a
                    href="#"
                    className="text-muted-foreground hover:text-foreground transition"
                  >
                    Features
                  </a>
                </li>
                <li>
                  <a
                    href="#"
                    className="text-muted-foreground hover:text-foreground transition"
                  >
                    Pricing
                  </a>
                </li>
                <li>
                  <a
                    href="#"
                    className="text-muted-foreground hover:text-foreground transition"
                  >
                    Documentation
                  </a>
                </li>
              </ul>
            </div>

            <div>
              <h4 className="font-semibold mb-4 text-gray-500">Company</h4>
              <ul className="space-y-2 text-sm">
                <li>
                  <a
                    href="#"
                    className="text-muted-foreground hover:text-foreground transition"
                  >
                    About
                  </a>
                </li>
                <li>
                  <a
                    href="#"
                    className="text-muted-foreground hover:text-foreground transition"
                  >
                    Blog
                  </a>
                </li>
                <li>
                  <a
                    href="#"
                    className="text-muted-foreground hover:text-foreground transition"
                  >
                    Contact
                  </a>
                </li>
              </ul>
            </div>

            <div>
              <h4 className="font-semibold mb-4 text-gray-500">Legal</h4>
              <ul className="space-y-2 text-sm">
                <li>
                  <a
                    href="#"
                    className="text-muted-foreground hover:text-foreground transition"
                  >
                    Privacy
                  </a>
                </li>
                <li>
                  <a
                    href="#"
                    className="text-muted-foreground hover:text-foreground transition"
                  >
                    Terms
                  </a>
                </li>
              </ul>
            </div>
          </div>

          <div className="border-t border-border pt-8">
            <p className="text-center text-sm text-muted-foreground">
              © {new Date().getFullYear()} Stellar Insights. All rights
              reserved.
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
