import React, { useState, useEffect } from 'react';
import { ChevronDown, AlertTriangle, Wifi, WifiOff } from 'lucide-react';

export interface NetworkInfo {
  network: 'mainnet' | 'testnet';
  display_name: string;
  rpc_url: string;
  horizon_url: string;
  network_passphrase: string;
  color: string;
  is_mainnet: boolean;
  is_testnet: boolean;
}

export interface NetworkSwitcherProps {
  className?: string;
  onNetworkChange?: (network: NetworkInfo) => void;
}

export function NetworkSwitcher({ className = '', onNetworkChange }: NetworkSwitcherProps) {
  const [currentNetwork, setCurrentNetwork] = useState<NetworkInfo | null>(null);
  const [availableNetworks, setAvailableNetworks] = useState<NetworkInfo[]>([]);
  const [isOpen, setIsOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showWarning, setShowWarning] = useState(false);
  const [pendingNetwork, setPendingNetwork] = useState<NetworkInfo | null>(null);

  // Fetch current network info and available networks
  useEffect(() => {
    const fetchNetworkInfo = async () => {
      try {
        setLoading(true);
        
        // Fetch current network
        const currentResponse = await fetch('/api/network/info');
        if (currentResponse.ok) {
          const current = await currentResponse.json();
          setCurrentNetwork(current);
        }

        // Fetch available networks
        const availableResponse = await fetch('/api/network/available');
        if (availableResponse.ok) {
          const available = await availableResponse.json();
          setAvailableNetworks(available);
        }
      } catch (err) {
        setError('Failed to load network information');
        console.error('Network info fetch error:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchNetworkInfo();
  }, []);

  const handleNetworkSelect = (network: NetworkInfo) => {
    if (network.network === currentNetwork?.network) {
      setIsOpen(false);
      return;
    }

    setPendingNetwork(network);
    setShowWarning(true);
    setIsOpen(false);
  };

  const confirmNetworkSwitch = async () => {
    if (!pendingNetwork) return;

    try {
      const response = await fetch('/api/network/switch', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          network: pendingNetwork.network,
        }),
      });

      if (response.ok) {
        const result = await response.json();
        
        // Show message about server restart requirement
        alert(result.message);
        
        // For now, we'll update the local state to show the intended network
        // In a real implementation, this would trigger a server restart
        setCurrentNetwork(pendingNetwork);
        onNetworkChange?.(pendingNetwork);
      } else {
        throw new Error('Failed to switch network');
      }
    } catch (err) {
      setError('Failed to switch network');
      console.error('Network switch error:', err);
    } finally {
      setShowWarning(false);
      setPendingNetwork(null);
    }
  };

  const cancelNetworkSwitch = () => {
    setShowWarning(false);
    setPendingNetwork(null);
  };

  if (loading) {
    return (
      <div className={`flex items-center space-x-2 ${className}`}>
        <div className="w-3 h-3 bg-gray-400 rounded-full animate-pulse" />
        <span className="text-sm text-muted-foreground">Loading...</span>
      </div>
    );
  }

  if (error || !currentNetwork) {
    return (
      <div className={`flex items-center space-x-2 ${className}`}>
        <WifiOff className="w-4 h-4 text-red-500" />
        <span className="text-sm text-red-500">Network Error</span>
      </div>
    );
  }

  return (
    <>
      <div className={`relative ${className}`}>
        <button
          onClick={() => setIsOpen(!isOpen)}
          className="flex items-center space-x-2 px-3 py-2 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
        >
          <div
            className="w-3 h-3 rounded-full"
            style={{ backgroundColor: currentNetwork.color }}
          />
          <span className="text-sm font-medium text-gray-900 dark:text-white">
            {currentNetwork.display_name}
          </span>
          <ChevronDown className={`w-4 h-4 text-muted-foreground transition-transform ${isOpen ? 'rotate-180' : ''}`} />
        </button>

        {isOpen && (
          <div className="absolute top-full left-0 mt-1 w-64 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg z-50">
            <div className="p-2">
              <div className="text-xs font-medium text-muted-foreground dark:text-muted-foreground uppercase tracking-wider mb-2 px-2">
                Available Networks
              </div>
              {availableNetworks.map((network) => (
                <button
                  key={network.network}
                  onClick={() => handleNetworkSelect(network)}
                  className={`w-full flex items-center space-x-3 px-3 py-2 rounded-md text-left hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors ${
                    network.network === currentNetwork.network
                      ? 'bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700'
                      : ''
                  }`}
                >
                  <div
                    className="w-3 h-3 rounded-full flex-shrink-0"
                    style={{ backgroundColor: network.color }}
                  />
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium text-gray-900 dark:text-white">
                      {network.display_name}
                    </div>
                    <div className="text-xs text-muted-foreground dark:text-muted-foreground truncate">
                      {network.horizon_url}
                    </div>
                  </div>
                  {network.network === currentNetwork.network && (
                    <Wifi className="w-4 h-4 text-green-500 flex-shrink-0" />
                  )}
                </button>
              ))}
            </div>
            
            <div className="border-t border-gray-200 dark:border-gray-700 p-3">
              <div className="text-xs text-muted-foreground dark:text-muted-foreground">
                <div className="flex items-center space-x-1 mb-1">
                  <span className="font-medium">Current:</span>
                  <span>{currentNetwork.display_name}</span>
                </div>
                <div className="text-xs opacity-75">
                  Switching networks will require a server restart
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Click outside to close */}
        {isOpen && (
          <div
            className="fixed inset-0 z-40"
            onClick={() => setIsOpen(false)}
          />
        )}
      </div>

      {/* Network Switch Warning Modal */}
      {showWarning && pendingNetwork && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4">
            <div className="flex items-center space-x-3 mb-4">
              <AlertTriangle className="w-6 h-6 text-amber-500" />
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                Switch Network
              </h3>
            </div>
            
            <div className="mb-6">
              <p className="text-muted-foreground dark:text-gray-300 mb-4">
                You are about to switch from{' '}
                <span className="font-medium">{currentNetwork.display_name}</span> to{' '}
                <span className="font-medium">{pendingNetwork.display_name}</span>.
              </p>
              
              <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700 rounded-lg p-3">
                <div className="text-sm text-amber-800 dark:text-amber-200">
                  <strong>Warning:</strong> This will switch all data and connections to the{' '}
                  {pendingNetwork.is_testnet ? 'testnet' : 'mainnet'}. The server will need to restart
                  to apply changes.
                </div>
              </div>
            </div>

            <div className="flex space-x-3">
              <button
                onClick={cancelNetworkSwitch}
                className="flex-1 px-4 py-2 text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={confirmNetworkSwitch}
                className="flex-1 px-4 py-2 text-white bg-blue-600 rounded-lg hover:bg-blue-700 transition-colors"
              >
                Switch Network
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}