import React from 'react';
import { Platform } from 'react-native';
import { useAppStore } from '@store/appStore';
import { StellarNetwork } from '@app-types/index';

export interface DialogState {
  isVisible: boolean;
  currentNetwork: StellarNetwork;
  targetNetwork: StellarNetwork;
  isLoading: boolean;
  error?: string;
}

export interface UseNetworkSwitchDialogResult extends DialogState {
  showDialog: () => void;
  hideDialog: () => void;
  switchNetwork: (network: StellarNetwork) => Promise<void>;
  resetError: () => void;
  canSwitch: boolean;
}

/**
 * Hook for managing network switch dialog state and logic
 * Handles dialog visibility, network switching with validation and error handling
 */
export function useNetworkSwitchDialog(): UseNetworkSwitchDialogResult {
  const { network: currentNetwork, setNetwork, isOnline } = useAppStore();
  const [state, setState] = React.useState<DialogState>({
    isVisible: false,
    currentNetwork,
    targetNetwork: currentNetwork,
    isLoading: false,
  });

  const canSwitch = isOnline && !state.isLoading;

  const showDialog = React.useCallback(() => {
    setState(prev => ({
      ...prev,
      isVisible: true,
      targetNetwork: currentNetwork,
      error: undefined,
    }));
  }, [currentNetwork]);

  const hideDialog = React.useCallback(() => {
    setState(prev => ({
      ...prev,
      isVisible: false,
      error: undefined,
    }));
  }, []);

  const resetError = React.useCallback(() => {
    setState(prev => ({
      ...prev,
      error: undefined,
    }));
  }, []);

  const switchNetwork = React.useCallback(
    async (targetNetwork: StellarNetwork) => {
      if (!canSwitch) {
        setState(prev => ({
          ...prev,
          error: isOnline ? 'Network switch in progress' : 'You are offline',
        }));
        return;
      }

      if (targetNetwork === currentNetwork) {
        setState(prev => ({
          ...prev,
          error: 'Already on this network',
        }));
        return;
      }

      setState(prev => ({
        ...prev,
        isLoading: true,
        targetNetwork,
        error: undefined,
      }));

      try {
        // Simulate network validation delay
        await new Promise(resolve => setTimeout(resolve, 800));

        // Update the store with new network
        setNetwork(targetNetwork);

        // Update local state
        setState(prev => ({
          ...prev,
          isLoading: false,
          currentNetwork: targetNetwork,
          isVisible: false,
          error: undefined,
        }));
      } catch (error) {
        const message = error instanceof Error ? error.message : 'Failed to switch network';
        setState(prev => ({
          ...prev,
          isLoading: false,
          error: message,
        }));
      }
    },
    [canSwitch, currentNetwork, isOnline, setNetwork]
  );

  // Update current network when app store changes
  React.useEffect(() => {
    setState(prev => ({
      ...prev,
      currentNetwork,
    }));
  }, [currentNetwork]);

  return {
    ...state,
    showDialog,
    hideDialog,
    switchNetwork,
    resetError,
    canSwitch,
  };
}

/**
 * Platform-specific configuration for dialog
 */
export const getDialogConfig = () => {
  return Platform.select({
    ios: {
      animationType: 'slide' as const,
      presentationStyle: 'pageSheet' as const,
    },
    android: {
      animationType: 'fade' as const,
      presentationStyle: 'overFullScreen' as const,
    },
    default: {
      animationType: 'slide' as const,
      presentationStyle: 'pageSheet' as const,
    },
  });
};
