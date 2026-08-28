import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import { StellarNetwork } from '@app-types/index';

interface AppState {
  theme: 'light' | 'dark';
  network: StellarNetwork;
  isOnline: boolean;
  isSyncing: boolean;
  setTheme: (theme: 'light' | 'dark') => void;
  setNetwork: (network: StellarNetwork) => void;
  setOnlineStatus: (isOnline: boolean) => void;
  setSyncStatus: (isSyncing: boolean) => void;
}

// subscribeWithSelector: src/hooks/useOfflineCaching.ts subscribes to just the
// `isOnline` slice via the (selector, listener) overload, which the base
// zustand store doesn't support -- without this, that subscription silently
// never fires (the "selector" is mistaken for the listener itself).
export const useAppStore = create<AppState>()(
  subscribeWithSelector(set => ({
    theme: 'light',
    network: 'testnet',
    isOnline: true,
    isSyncing: false,
    setTheme: theme => set({ theme }),
    setNetwork: network => set({ network }),
    setOnlineStatus: isOnline => set({ isOnline }),
    setSyncStatus: isSyncing => set({ isSyncing }),
  })),
);
