import { useState } from "react";
import { Link } from "react-router-dom";
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
import { useWallet } from "@/contexts/WalletContext";

export default function Landing() {
  const { isConnected, address, isConnecting, connectWallet, disconnectWallet } = useWallet();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [showWalletMenu, setShowWalletMenu] = useState(false);

  return (
    <div className="min-h-screen bg-background text-foreground font-sans">
      {/* Navigation */}
      <nav className="fixed w-full top-0 z-50 bg-background/80 backdrop-blur-sm border-b border-border">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                <TrendingUp className="w-5 h-5 text-primary-foreground" />
              </div>
              <span className="text-xl font-bold">Stellar Insights</span>
            </div>

            {/* Desktop Navigation */}
            <div className="hidden md:flex items-center gap-8">
              <a href="#problem" className="text-sm font-medium text-muted-foreground hover:text-primary transition-colors">
                Problem
              </a>
              <a href="#solution" className="text-sm font-medium text-muted-foreground hover:text-primary transition-colors">
                Solution
              </a>
              <a href="#features" className="text-sm font-medium text-muted-foreground hover:text-primary transition-colors">
                Features
              </a>
              <Link to="/corridors" className="text-sm font-medium text-muted-foreground hover:text-primary transition-colors">
                Corridors
              </Link>

              {isConnected && address ? (
                <div className="relative">
                  <button
                    onClick={() => setShowWalletMenu(!showWalletMenu)}
                    className="px-4 py-2 bg-primary text-primary-foreground rounded-full text-sm font-medium hover:opacity-90 transition flex items-center gap-2"
                  >
                    <Wallet className="w-4 h-4" />
                    <span className="font-mono">{address.slice(0, 6)}...{address.slice(-4)}</span>
                  </button>

                  {showWalletMenu && (
                    <div className="absolute right-0 mt-2 w-48 bg-card border border-border rounded-lg shadow-lg py-2 z-50">
                      <div className="px-4 py-2 text-xs text-muted-foreground border-b border-border font-mono truncate">
                        {address}
                      </div>
                      <button
                        onClick={() => {
                          disconnectWallet();
                          setShowWalletMenu(false);
                        }}
                        className="w-full text-left px-4 py-2 text-sm hover:bg-muted transition"
                      >
                        Disconnect
                      </button>
                    </div>
                  )}
                </div>
              ) : (
                <button
                  onClick={connectWallet}
                  disabled={isConnecting}
                  className="px-4 py-2 bg-primary text-primary-foreground rounded-full text-sm font-medium hover:opacity-90 transition disabled:opacity-50 flex items-center gap-2"
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
              {mobileMenuOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
            </button>
          </div>

          {/* Mobile Navigation */}
          {mobileMenuOpen && (
            <div className="md:hidden pb-4 space-y-3">
              <a href="#problem" className="block text-sm font-medium text-muted-foreground hover:text-primary">
                Problem
              </a>
              <a href="#solution" className="block text-sm font-medium text-muted-foreground hover:text-primary">
                Solution
              </a>
              <a href="#features" className="block text-sm font-medium text-muted-foreground hover:text-primary">
                Features
              </a>
              <Link to="/corridors" className="block text-sm font-medium text-muted-foreground hover:text-primary">
                Corridors
              </Link>

              {isConnected && address ? (
                <button
                  onClick={() => {
                    disconnectWallet();
                    setMobileMenuOpen(false);
                  }}
                  className="w-full px-4 py-2 bg-primary text-primary-foreground rounded-full text-sm font-medium"
                >
                  Disconnect: {address.slice(0, 6)}...{address.slice(-4)}
                </button>
              ) : (
                <button
                  onClick={() => {
                    connectWallet();
                    setMobileMenuOpen(false);
                  }}
                  disabled={isConnecting}
                  className="w-full px-4 py-2 bg-primary text-primary-foreground rounded-full text-sm font-medium disabled:opacity-50 flex items-center justify-center gap-2"
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
          <div className="absolute top-20 right-0 w-96 h-96 bg-primary/5 rounded-full blur-3xl -z-10" />

          <div className="space-y-8 text-center">
            <div className="inline-block px-4 py-2 bg-primary/10 border border-primary/20 rounded-full">
              <span className="text-primary font-medium text-sm">Stellar Payment Intelligence</span>
            </div>

            <h1 className="text-4xl sm:text-5xl lg:text-6xl font-bold tracking-tight">
              Payment Reliability,
              <br />
              <span className="text-primary">Not Just Speed</span>
            </h1>

            <p className="text-lg sm:text-xl text-muted-foreground max-w-2xl mx-auto">
              Deep insights into Stellar payment network performance. Predict
              success, optimize routing, and identify liquidity bottlenecks
              before they impact your business.
            </p>

            <div className="flex flex-col sm:flex-row gap-4 justify-center pt-4">
              {isConnected ? (
                <>
                  <Link
                    to="/corridors"
                    className="px-8 py-4 bg-primary text-primary-foreground rounded-full font-semibold hover:opacity-90 transition flex items-center justify-center gap-2"
                  >
                    Access Dashboard <ArrowRight className="w-5 h-5" />
                  </Link>
                  <button className="px-8 py-4 bg-secondary text-secondary-foreground rounded-full font-semibold hover:opacity-90 transition">
                    View Documentation
                  </button>
                </>
              ) : (
                <>
                  <button
                    onClick={connectWallet}
                    disabled={isConnecting}
                    className="px-8 py-4 bg-primary text-primary-foreground rounded-full font-semibold hover:opacity-90 transition disabled:opacity-50 flex items-center justify-center gap-2"
                  >
                    <Wallet className="w-5 h-5" />
                    {isConnecting ? "Connecting..." : "Connect Wallet to Start"}
                    {!isConnecting && <ArrowRight className="w-5 h-5" />}
                  </button>
                  <button className="px-8 py-4 bg-secondary text-secondary-foreground rounded-full font-semibold hover:opacity-90 transition">
                    Watch Demo
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* Problem Section */}
      <section id="problem" className="py-20 px-4 sm:px-6 lg:px-8 bg-muted/30">
        <div className="max-w-6xl mx-auto">
          <div className="text-center mb-16">
            <h2 className="text-3xl sm:text-4xl font-bold mb-4">The Challenge</h2>
            <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
              Stellar is powerful for stablecoins, payments, and asset issuance,
              but speed alone isn't enough
            </p>
          </div>

          <div className="grid md:grid-cols-2 gap-6">
            {[
              {
                icon: <TrendingUp className="w-6 h-6" />,
                title: "Asset Corridor Reliability",
                description: "Which payment corridors consistently succeed vs fail? Current tools don't answer this.",
              },
              {
                icon: <AlertCircle className="w-6 h-6" />,
                title: "Liquidity Uncertainty",
                description: "Is there enough liquidity in major payment paths? Bottlenecks appear without warning.",
              },
              {
                icon: <BarChart3 className="w-6 h-6" />,
                title: "Anchor & Asset Bottlenecks",
                description: "Certain anchors or assets block transfers. Performance data is hard to access.",
              },
              {
                icon: <Zap className="w-6 h-6" />,
                title: "Market Efficiency Unknown",
                description: "Are markets efficient, stable, or unreliable under stress? Raw transactions don't tell the story.",
              },
            ].map((item, i) => (
              <div
                key={i}
                className="p-6 rounded-xl bg-card border border-border hover:border-primary/50 transition group"
              >
                <div className="w-12 h-12 bg-primary/10 rounded-lg flex items-center justify-center text-primary mb-4 group-hover:bg-primary/20 transition">
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
            <h2 className="text-3xl sm:text-4xl font-bold mb-4">The Solution</h2>
            <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
              Stellar Insights Intelligence Dashboard for real-world payment network visibility
            </p>
          </div>

          <div className="space-y-12">
            {[
              {
                title: "Predict Payment Success",
                description: "Know the likelihood of payment success before sending. Make intelligent routing decisions based on real-time network data.",
                icon: <CheckCircle2 className="w-8 h-8" />,
              },
              {
                title: "Optimize Routing Paths",
                description: "Discover the most reliable paths through the network. Avoid bottlenecks and ensure payments reach their destination.",
                icon: <TrendingUp className="w-8 h-8" />,
              },
              {
                title: "Quantify Real-World Reliability",
                description: "Move beyond TPS metrics. Understand actual payment success rates, latency, and failure patterns across corridors.",
                icon: <BarChart3 className="w-8 h-8" />,
              },
              {
                title: "Identify Liquidity Bottlenecks",
                description: "See where liquidity is constrained. Improve provisioning strategies with data-driven insights into market gaps.",
                icon: <Zap className="w-8 h-8" />,
              },
              {
                title: "Understand Ecosystem Health",
                description: "Track network risk and health trends. Anticipate problems before they impact payments and make proactive decisions.",
                icon: <AlertCircle className="w-8 h-8" />,
              },
            ].map((item, i) => (
              <div key={i} className="flex flex-col md:flex-row gap-8 items-center">
                <div className="shrink-0 w-16 h-16 bg-primary/10 rounded-xl flex items-center justify-center text-primary">
                  {item.icon}
                </div>
                <div className="flex-1 text-center md:text-left">
                  <h3 className="text-2xl font-bold mb-2">{item.title}</h3>
                  <p className="text-lg text-muted-foreground">{item.description}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features */}
      <section id="features" className="py-20 px-4 sm:px-6 lg:px-8 bg-muted/30">
        <div className="max-w-6xl mx-auto">
          <div className="text-center mb-16">
            <h2 className="text-3xl sm:text-4xl font-bold mb-4">Built for Your Role</h2>
            <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
              Whether you run a wallet, build on Stellar, operate an anchor, or manage a business, we have insights for you
            </p>
          </div>

          <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6">
            {[
              { role: "Wallets", benefit: "Show users payment reliability before confirmation" },
              { role: "Developers", benefit: "Build smarter payment logic with network intelligence" },
              { role: "Businesses", benefit: "Ensure payment success in cross-border operations" },
              { role: "Anchors", benefit: "Monitor liquidity provisioning and optimize reserves" },
            ].map((item, i) => (
              <div
                key={i}
                className="p-6 rounded-lg bg-card border border-border hover:border-primary/50 transition cursor-pointer"
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
          <div className="bg-primary/10 border border-primary/20 rounded-2xl p-12 text-center">
            <h2 className="text-3xl sm:text-4xl font-bold mb-4">
              {isConnected ? "Welcome to Stellar Insights" : "Ready to See the Whole Picture?"}
            </h2>
            <p className="text-lg text-muted-foreground mb-8">
              {isConnected
                ? `Connected with ${address?.slice(0, 6)}...${address?.slice(-4)}. Explore deep insights into Stellar payment network performance.`
                : "Connect your Stellar wallet to unlock deep insights into payment network performance and liquidity health."}
            </p>

            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              {isConnected ? (
                <>
                  <Link
                    to="/corridors"
                    className="px-8 py-3 bg-primary text-primary-foreground rounded-full font-semibold hover:opacity-90 transition"
                  >
                    Go to Dashboard
                  </Link>
                  <button className="px-8 py-3 bg-secondary text-secondary-foreground rounded-full font-semibold hover:opacity-90 transition">
                    View API Docs
                  </button>
                </>
              ) : (
                <button
                  onClick={connectWallet}
                  disabled={isConnecting}
                  className="px-8 py-3 bg-primary text-primary-foreground rounded-full font-semibold hover:opacity-90 transition disabled:opacity-50 flex items-center justify-center gap-2"
                >
                  <Wallet className="w-5 h-5" />
                  {isConnecting ? "Connecting..." : "Connect Wallet"}
                </button>
              )}
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="py-12 px-4 sm:px-6 lg:px-8 border-t border-border">
        <div className="max-w-6xl mx-auto">
          <div className="flex flex-col md:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
                <TrendingUp className="w-5 h-5 text-primary-foreground" />
              </div>
              <span className="text-lg font-bold">Stellar Insights</span>
            </div>
            <p className="text-sm text-muted-foreground">
              © 2024 Stellar Insights. Built for the Stellar ecosystem.
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
