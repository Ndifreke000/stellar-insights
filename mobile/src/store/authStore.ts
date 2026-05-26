import { create } from 'zustand';
import { User, AuthTokens } from '@appTypes/index';

interface AuthState {
  user: User | null;
  tokens: AuthTokens | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  setUser: (user: User | null) => void;
  setTokens: (tokens: AuthTokens | null) => void;
  setLoading: (isLoading: boolean) => void;
  logout: () => void;
}

export const useAuthStore = create<AuthState>(set => ({
  user: null,
  tokens: null,
  isAuthenticated: false,
  isLoading: true,
  setUser: user => set({ user, isAuthenticated: !!user }),
  setTokens: tokens => set({ tokens }),
  setLoading: isLoading => set({ isLoading }),
  logout: () => set({ user: null, tokens: null, isAuthenticated: false }),
}));
