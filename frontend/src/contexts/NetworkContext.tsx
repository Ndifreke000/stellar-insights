'use client';

import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
} from 'react';
import type { NetworkInfo } from '@/lib/api/types';
import { logger } from '@/lib/logger';

interface NetworkContextType {
  network: NetworkInfo | null;
  loading: boolean;
  setNetwork: (network: NetworkInfo) => void;
  refreshNetwork: () => Promise<void>;
}

const NetworkContext = createContext<NetworkContextType | undefined>(undefined);

export function stellarExpertAccountUrl(
  network: NetworkInfo | null | undefined,
  account: string,
): string {
  const segment = network?.is_testnet ? 'testnet' : 'public';
  return `https://stellar.expert/explorer/${segment}/account/${account}`;
}

export function stellarExpertContractUrl(
  network: NetworkInfo | null | undefined,
  contractId: string,
): string {
  const segment = network?.is_testnet ? 'testnet' : 'public';
  return `https://stellar.expert/explorer/${segment}/contract/${contractId}`;
}

interface NetworkProviderProps {
  children: React.ReactNode;
}

export function NetworkProvider({ children }: NetworkProviderProps) {
  const [network, setNetworkState] = useState<NetworkInfo | null>(null);
  const [loading, setLoading] = useState(true);

  const refreshNetwork = useCallback(async () => {
    try {
      setLoading(true);
      const response = await fetch('/api/network/info');
      if (response.ok) {
        const current = (await response.json()) as NetworkInfo;
        setNetworkState(current);
      }
    } catch (err) {
      logger.error('Failed to refresh network info:', err as string);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshNetwork();
  }, [refreshNetwork]);

  const setNetwork = useCallback((next: NetworkInfo) => {
    setNetworkState(next);
  }, []);

  return (
    <NetworkContext.Provider
      value={{ network, loading, setNetwork, refreshNetwork }}
    >
      {children}
    </NetworkContext.Provider>
  );
}

export function useNetwork(): NetworkContextType {
  const ctx = useContext(NetworkContext);
  if (!ctx) {
    throw new Error('useNetwork must be used within a NetworkProvider');
  }
  return ctx;
}
