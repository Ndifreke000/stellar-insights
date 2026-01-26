import React, { createContext, useContext, useState, useCallback, useEffect } from 'react';

interface WalletContextType {
  isConnected: boolean;
  address: string | null;
  isConnecting: boolean;
  connectWallet: () => Promise<void>;
  disconnectWallet: () => void;
}

const WalletContext = createContext<WalletContextType | undefined>(undefined);

export function WalletProvider({ children }: { children: React.ReactNode }) {
  const [isConnected, setIsConnected] = useState(false);
  const [address, setAddress] = useState<string | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);

  useEffect(() => {
    const checkWalletConnection = async () => {
      try {
        const savedAddress = localStorage.getItem('stellar_wallet_address');
        if (savedAddress) {
          setAddress(savedAddress);
          setIsConnected(true);
        }
      } catch (error) {
        console.error('Error checking wallet connection:', error);
      }
    };

    checkWalletConnection();
  }, []);

  const connectWallet = useCallback(async () => {
    setIsConnecting(true);
    try {
      if (typeof window !== 'undefined' && (window as any).stellar) {
        const result = await (window as any).stellar.requestPublicKey();
        if (result) {
          setAddress(result);
          setIsConnected(true);
          localStorage.setItem('stellar_wallet_address', result);
        }
      } else {
        const demoAddress = `G${Math.random().toString(36).substring(2, 15)}${Math.random().toString(36).substring(2, 15)}`;
        setAddress(demoAddress);
        setIsConnected(true);
        localStorage.setItem('stellar_wallet_address', demoAddress);
      }
    } catch (error) {
      console.error('Error connecting wallet:', error);
      setIsConnecting(false);
      throw error;
    } finally {
      setIsConnecting(false);
    }
  }, []);

  const disconnectWallet = useCallback(() => {
    setAddress(null);
    setIsConnected(false);
    localStorage.removeItem('stellar_wallet_address');
  }, []);

  return (
    <WalletContext.Provider value={{ isConnected, address, isConnecting, connectWallet, disconnectWallet }}>
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
}
