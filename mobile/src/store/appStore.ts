import { create } from 'zustand';
import { StellarNetwork } from '@appTypes/index';

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

export const useAppStore = create<AppState>(set => ({
  theme: 'light',
  network: 'testnet',
  isOnline: true,
  isSyncing: false,
  setTheme: theme => set({ theme }),
  setNetwork: network => set({ network }),
  setOnlineStatus: isOnline => set({ isOnline }),
  setSyncStatus: isSyncing => set({ isSyncing }),
}));
