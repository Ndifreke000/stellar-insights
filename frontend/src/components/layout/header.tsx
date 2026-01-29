"use client";

import React, { useEffect, useRef } from "react";
import { Menu, Wallet, LogOut } from "lucide-react";
import { useWallet } from "../lib/wallet-context";
import { useNotifications } from "../../contexts/NotificationContext";

interface HeaderProps {
  onMenuToggle: () => void;
  sidebarOpen: boolean;
}

export function Header({ onMenuToggle, sidebarOpen }: HeaderProps) {
  const { isConnected, address, connectWallet, disconnectWallet } = useWallet();
  const { showToast } = useNotifications();
  const [showWalletMenu, setShowWalletMenu] = React.useState(false);
  const prevConnectedRef = useRef<boolean | null>(null);

  const displayAddress = address
    ? `${address.slice(0, 6)}...${address.slice(-4)}`
    : null;

  // Show notification when wallet connection status changes
  useEffect(() => {
    // Skip the first render to avoid showing notification on initial load
    if (prevConnectedRef.current === null) {
      prevConnectedRef.current = isConnected;
      return;
    }

    // Only show notification if the connection status actually changed
    if (prevConnectedRef.current !== isConnected) {
      const currentDisplayAddress = address
        ? `${address.slice(0, 6)}...${address.slice(-4)}`
        : null;

      if (isConnected && address) {
        showToast({
          type: 'success',
          priority: 'medium',
          title: 'Wallet Connected',
          message: `Successfully connected to ${currentDisplayAddress}`,
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
  }, [isConnected, address, showToast]); // Remove displayAddress from dependencies

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
    <header className="fixed top-0 left-0 right-0 h-16 bg-white dark:bg-slate-900 border-b border-gray-200 dark:border-slate-700 z-40">
      <div className="flex items-center justify-between h-full px-4 sm:px-6">
        {/* Left: Menu Toggle & Logo */}
        <div className="flex items-center gap-4">
          <button
            onClick={onMenuToggle}
            className="p-2 hover:bg-gray-100 dark:hover:bg-slate-800 rounded-lg transition-colors lg:hidden min-w-[44px] min-h-[44px] touch-manipulation active:bg-gray-200 dark:active:bg-slate-700"
            aria-label={sidebarOpen ? "Close sidebar" : "Open sidebar"}
            aria-expanded={sidebarOpen}
          >
            <Menu className="w-6 h-6" />
          </button>
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 bg-blue-500 rounded-lg flex items-center justify-center">
              <TrendingUpIcon className="w-5 h-5 text-white" />
            </div>
            <span className="text-lg font-bold text-gray-900 dark:text-white hidden sm:inline">
              Stellar Insights
            </span>
          </div>
        </div>

        {/* Right: Wallet Connect */}
        <div className="relative">
          {isConnected ? (
            <div className="flex items-center gap-3">
              <div className="hidden sm:flex items-center gap-2 px-3 py-2 bg-gray-50 dark:bg-slate-800 rounded-lg">
                <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                <span className="text-sm text-gray-700 dark:text-gray-300">
                  {displayAddress}
                </span>
              </div>
              <button
                onClick={() => setShowWalletMenu(!showWalletMenu)}
                className="p-2 hover:bg-gray-100 dark:hover:bg-slate-800 rounded-lg transition-colors min-w-[44px] min-h-[44px] touch-manipulation active:bg-gray-200 dark:active:bg-slate-700"
                aria-label="Wallet menu"
                aria-expanded={showWalletMenu}
              >
                <Wallet className="w-6 h-6" />
              </button>

              {showWalletMenu && (
                <div className="absolute right-0 top-full mt-2 bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 rounded-lg shadow-lg py-2 min-w-[180px]">
                  <button
                    onClick={handleDisconnect}
                    className="w-full px-4 py-2 flex items-center gap-2 text-sm hover:bg-gray-100 dark:hover:bg-slate-700 text-gray-700 dark:text-gray-300 transition-colors"
                  >
                    <LogOut className="w-4 h-4" />
                    Disconnect
                  </button>
                </div>
              )}
            </div>
          ) : (
            <button
              onClick={handleConnect}
              className="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors font-medium text-sm flex items-center gap-2 min-h-[44px] touch-manipulation active:bg-blue-700"
            >
              <Wallet className="w-4 h-4" />
              <span className="hidden sm:inline">Connect Wallet</span>
              <span className="sm:hidden">Connect</span>
            </button>
          )}
        </div>
      </div>
    </header>
  );
}

// Inline icon to avoid extra import
function TrendingUpIcon(props: React.SVGProps<SVGSVGElement>) {
  return (
    <svg
      {...props}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"
      />
    </svg>
  );
}
