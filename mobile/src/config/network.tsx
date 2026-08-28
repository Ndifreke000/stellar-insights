import React, { createContext, useState, useEffect, useContext } from 'react';
import AsyncStorage from '@react-native-async-storage/async-storage';
import { STORAGE_KEYS } from './constants';

export interface NetworkConfig {
  id: 'testnet' | 'mainnet';
  name: string;
  apiBaseUrl: string;
  horizonUrl: string;
  networkPassphrase: string;
}

export const NETWORKS: Record<'testnet' | 'mainnet', NetworkConfig> = {
  testnet: {
    id: 'testnet',
    name: 'Testnet',
    apiBaseUrl: process.env.EXPO_PUBLIC_TESTNET_API_URL || 'http://localhost:3000',
    horizonUrl: 'https://horizon-testnet.stellar.org',
    networkPassphrase: 'Test SDF Network ; September 2015',
  },
  mainnet: {
    id: 'mainnet',
    name: 'Mainnet',
    apiBaseUrl: process.env.EXPO_PUBLIC_MAINNET_API_URL || 'https://api.stellar-insights.com',
    horizonUrl: 'https://horizon.stellar.org',
    networkPassphrase: 'Public Global Stellar Network ; September 2015',
  },
};

const DEFAULT_NETWORK_ID = (process.env.EXPO_PUBLIC_DEFAULT_NETWORK || 'testnet') as 'testnet' | 'mainnet';

export async function getCurrentNetwork(): Promise<NetworkConfig> {
  try {
    const stored = await AsyncStorage.getItem(STORAGE_KEYS.NETWORK_PREFERENCE);
    if (stored === 'mainnet' || stored === 'testnet') {
      return NETWORKS[stored];
    }
  } catch (error) {
    console.error('Failed to get network preference:', error);
  }
  return NETWORKS[DEFAULT_NETWORK_ID];
}

export interface NetworkContextType {
  currentNetwork: NetworkConfig;
  setNetwork: (networkId: 'testnet' | 'mainnet') => Promise<void>;
  isLoading: boolean;
}

export const NetworkContext = createContext<NetworkContextType | undefined>(undefined);

export const useNetwork = () => {
  const context = useContext(NetworkContext);
  if (!context) {
    throw new Error('useNetwork must be used within a NetworkProvider');
  }
  return context;
};

export const NetworkProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [currentNetwork, setCurrentNetworkState] = useState<NetworkConfig>(NETWORKS[DEFAULT_NETWORK_ID]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    async function loadNetwork() {
      const net = await getCurrentNetwork();
      setCurrentNetworkState(net);
      setIsLoading(false);
    }
    loadNetwork();
  }, []);

  const setNetwork = async (networkId: 'testnet' | 'mainnet') => {
    try {
      await AsyncStorage.setItem(STORAGE_KEYS.NETWORK_PREFERENCE, networkId);
      setCurrentNetworkState(NETWORKS[networkId]);
    } catch (error) {
      console.error('Failed to save network preference:', error);
    }
  };

  return (
    <NetworkContext.Provider value={{ currentNetwork, setNetwork, isLoading }}>
      {children}
    </NetworkContext.Provider>
  );
};
