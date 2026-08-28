import React from 'react';
import {
  ActivityIndicator,
  Alert,
  Modal,
  Platform,
  Pressable,
  SafeAreaView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { useNetworkSwitchDialog, getDialogConfig } from '@hooks/useNetworkSwitchDialog';
import { StellarNetwork } from '@app-types/index';
import { useAppStore } from '@store/appStore';

export interface NetworkSwitchDialogProps {
  visible: boolean;
  onDismiss?: () => void;
  testID?: string;
}

/**
 * Network Switch Dialog component
 * Displays a modal for switching between Stellar networks (testnet/mainnet)
 * with platform-specific optimizations and error handling
 */
export const NetworkSwitchDialog: React.FC<NetworkSwitchDialogProps> = ({
  visible,
  onDismiss,
  testID,
}) => {
  const {
    showDialog,
    hideDialog,
    switchNetwork,
    currentNetwork,
    isLoading,
    error,
    canSwitch,
    resetError,
  } = useNetworkSwitchDialog();
  const { isOnline } = useAppStore();
  const dialogConfig = getDialogConfig();

  const handleClose = React.useCallback(() => {
    hideDialog();
    onDismiss?.();
  }, [hideDialog, onDismiss]);

  const handleNetworkSwitch = React.useCallback(
    async (network: StellarNetwork) => {
      try {
        await switchNetwork(network);
        Alert.alert('Success', `Switched to ${network} network`);
      } catch (err) {
        Alert.alert('Error', `Failed to switch to ${network} network`);
      }
    },
    [switchNetwork]
  );

  React.useEffect(() => {
    if (visible) {
      showDialog();
    }
  }, [visible, showDialog]);

  const networks: StellarNetwork[] = ['testnet', 'mainnet'];

  return (
    <Modal
      visible={visible}
      animationType={dialogConfig?.animationType || 'slide'}
      presentationStyle={dialogConfig?.presentationStyle || 'pageSheet'}
      transparent={false}
      testID={testID}
      onRequestClose={handleClose}>
      <SafeAreaView style={styles.container} accessibilityLabel="Network switch dialog">
        {/* Header */}
        <View style={styles.header}>
          <Text style={styles.title}>Switch Network</Text>
          <Text style={styles.subtitle}>Select a Stellar network to connect to</Text>
        </View>

        {/* Error Message */}
        {error && (
          <View
            style={styles.errorContainer}
            accessibilityRole="alert"
            accessibilityLabel={`Error: ${error}`}>
            <Text style={styles.errorIcon}>⚠️</Text>
            <Text style={styles.errorText}>{error}</Text>
            {error !== 'You are offline' && (
              <Pressable onPress={resetError} accessibilityRole="button">
                <Text style={styles.errorDismiss}>✕</Text>
              </Pressable>
            )}
          </View>
        )}

        {/* Offline Warning */}
        {!isOnline && (
          <View style={styles.offlineWarning} accessibilityRole="alert">
            <Text style={styles.warningIcon}>📡</Text>
            <View style={{ flex: 1 }}>
              <Text style={styles.warningTitle}>You are offline</Text>
              <Text style={styles.warningText}>Network switching is disabled while offline</Text>
            </View>
          </View>
        )}

        {/* Network Options */}
        <View style={styles.networkListContainer}>
          <Text style={styles.optionTitle}>Available Networks</Text>

          {networks.map(network => (
            <Pressable
              key={network}
              style={({ pressed }) => [
                styles.networkOption,
                {
                  backgroundColor:
                    currentNetwork === network ? '#E3F2FD' : pressed ? '#F5F5F5' : '#FFFFFF',
                  opacity: isLoading && currentNetwork !== network ? 0.5 : 1,
                },
              ]}
              onPress={() => handleNetworkSwitch(network)}
              disabled={isLoading || !canSwitch}
              accessibilityRole="radio"
              accessibilityState={{
                selected: currentNetwork === network,
                disabled: isLoading || !canSwitch,
              }}
              accessibilityLabel={`${network} network option`}>
              <View style={styles.networkContent}>
                <View style={styles.networkIcon}>
                  <Text style={styles.networkIconText}>{network === 'testnet' ? '🧪' : '🚀'}</Text>
                </View>

                <View style={{ flex: 1 }}>
                  <Text style={styles.networkName}>{network.toUpperCase()}</Text>
                  <Text style={styles.networkDescription}>
                    {network === 'testnet'
                      ? 'Testing and development network'
                      : 'Production network'}
                  </Text>
                </View>

                {currentNetwork === network && (
                  <View style={styles.selectedBadge}>
                    <Text style={styles.selectedBadgeText}>✓</Text>
                  </View>
                )}

                {isLoading && currentNetwork === network && (
                  <ActivityIndicator
                    size="small"
                    color="#0066CC"
                    accessibilityLabel={`Switching to ${network}`}
                  />
                )}
              </View>
            </Pressable>
          ))}
        </View>

        {/* Network Info */}
        <View style={styles.infoSection}>
          <Text style={styles.sectionTitle}>Network Information</Text>

          <View style={styles.infoCard}>
            <Text style={styles.infoLabel}>Current Network</Text>
            <Text style={styles.infoValue}>{currentNetwork.toUpperCase()}</Text>
          </View>

          <View style={styles.infoCard}>
            <Text style={styles.infoLabel}>Connection Status</Text>
            <View style={styles.statusRow}>
              <View
                style={[styles.statusDot, { backgroundColor: isOnline ? '#4CAF50' : '#FF9800' }]}
              />
              <Text style={styles.infoValue}>{isOnline ? 'Online' : 'Offline'}</Text>
            </View>
          </View>

          <View style={styles.infoCard}>
            <Text style={styles.infoLabel}>Switch Status</Text>
            <Text style={styles.infoValue}>{isLoading ? 'Switching...' : 'Ready'}</Text>
          </View>
        </View>

        {/* Footer Buttons */}
        <View style={styles.footer}>
          <Pressable
            style={({ pressed }) => [
              styles.button,
              styles.cancelButton,
              { opacity: pressed ? 0.7 : 1 },
            ]}
            onPress={handleClose}
            disabled={isLoading}
            accessibilityRole="button"
            accessibilityLabel="Close dialog">
            <Text style={styles.cancelButtonText}>Close</Text>
          </Pressable>
        </View>
      </SafeAreaView>
    </Modal>
  );
};

/**
 * Network Switch Button component
 * Button that opens the network switch dialog
 */
export interface NetworkSwitchButtonProps {
  onPress?: () => void;
  testID?: string;
}

export const NetworkSwitchButton: React.FC<NetworkSwitchButtonProps> = ({ onPress, testID }) => {
  const { network } = useAppStore();
  const [showDialog, setShowDialog] = React.useState(false);

  return (
    <>
      <Pressable
        style={({ pressed }) => [styles.switchButton, { opacity: pressed ? 0.7 : 1 }]}
        onPress={() => setShowDialog(true)}
        accessibilityRole="button"
        accessibilityLabel={`Switch network, currently on ${network}`}
        testID={testID}>
        <Text style={styles.switchButtonIcon}>🌐</Text>
        <Text style={styles.switchButtonText}>{network.toUpperCase()}</Text>
      </Pressable>

      <NetworkSwitchDialog visible={showDialog} onDismiss={() => setShowDialog(false)} />
    </>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#FAFAFA',
  },
  header: {
    paddingHorizontal: 20,
    paddingTop: 16,
    paddingBottom: 8,
  },
  title: {
    fontSize: 28,
    fontWeight: '700',
    marginBottom: 4,
    color: '#212121',
  },
  subtitle: {
    fontSize: 14,
    color: '#666666',
  },
  errorContainer: {
    marginHorizontal: 16,
    marginTop: 12,
    marginBottom: 8,
    paddingHorizontal: 12,
    paddingVertical: 10,
    backgroundColor: '#FFEBEE',
    borderRadius: 8,
    borderLeftWidth: 4,
    borderLeftColor: '#D32F2F',
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
  },
  errorIcon: {
    fontSize: 18,
  },
  errorText: {
    flex: 1,
    color: '#C62828',
    fontWeight: '500',
    fontSize: 13,
  },
  errorDismiss: {
    fontSize: 18,
    color: '#C62828',
    fontWeight: 'bold',
  },
  offlineWarning: {
    marginHorizontal: 16,
    marginBottom: 12,
    paddingHorizontal: 12,
    paddingVertical: 10,
    backgroundColor: '#FFF3E0',
    borderRadius: 8,
    borderLeftWidth: 4,
    borderLeftColor: '#F57C00',
    flexDirection: 'row',
    alignItems: 'flex-start',
    gap: 8,
  },
  warningIcon: {
    fontSize: 18,
    marginTop: 2,
  },
  warningTitle: {
    color: '#E65100',
    fontWeight: '600',
    fontSize: 13,
    marginBottom: 2,
  },
  warningText: {
    color: '#E65100',
    fontSize: 12,
  },
  networkListContainer: {
    paddingHorizontal: 16,
    paddingVertical: 12,
    flex: 1,
  },
  optionTitle: {
    fontSize: 14,
    fontWeight: '600',
    color: '#212121',
    marginBottom: 12,
  },
  networkOption: {
    borderRadius: 12,
    padding: 12,
    marginBottom: 10,
    borderWidth: 2,
    borderColor: '#E0E0E0',
  },
  networkContent: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 12,
  },
  networkIcon: {
    width: 48,
    height: 48,
    borderRadius: 24,
    backgroundColor: '#E3F2FD',
    justifyContent: 'center',
    alignItems: 'center',
  },
  networkIconText: {
    fontSize: 24,
  },
  networkName: {
    fontSize: 16,
    fontWeight: '600',
    color: '#212121',
    marginBottom: 2,
  },
  networkDescription: {
    fontSize: 12,
    color: '#999999',
  },
  selectedBadge: {
    width: 28,
    height: 28,
    borderRadius: 14,
    backgroundColor: '#4CAF50',
    justifyContent: 'center',
    alignItems: 'center',
  },
  selectedBadgeText: {
    color: '#FFFFFF',
    fontWeight: 'bold',
    fontSize: 16,
  },
  infoSection: {
    paddingHorizontal: 16,
    paddingBottom: 12,
  },
  sectionTitle: {
    fontSize: 14,
    fontWeight: '600',
    color: '#212121',
    marginBottom: 10,
  },
  infoCard: {
    backgroundColor: '#FFFFFF',
    borderRadius: 8,
    padding: 12,
    marginBottom: 8,
    borderWidth: 1,
    borderColor: '#E0E0E0',
  },
  infoLabel: {
    fontSize: 12,
    color: '#999999',
    fontWeight: '500',
    marginBottom: 4,
  },
  infoValue: {
    fontSize: 14,
    fontWeight: '600',
    color: '#212121',
  },
  statusRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
  },
  statusDot: {
    width: 8,
    height: 8,
    borderRadius: 4,
  },
  footer: {
    paddingHorizontal: 16,
    paddingBottom: Platform.select({ ios: 16, android: 12, default: 12 }),
    gap: 8,
  },
  button: {
    paddingVertical: 12,
    paddingHorizontal: 16,
    borderRadius: 8,
    alignItems: 'center',
    justifyContent: 'center',
  },
  cancelButton: {
    backgroundColor: '#E0E0E0',
  },
  cancelButtonText: {
    color: '#212121',
    fontSize: 14,
    fontWeight: '600',
  },
  // Switch button styles
  switchButton: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 6,
    paddingHorizontal: 12,
    paddingVertical: 8,
    backgroundColor: '#E3F2FD',
    borderRadius: 8,
    borderWidth: 1,
    borderColor: '#BBDEFB',
  },
  switchButtonIcon: {
    fontSize: 16,
  },
  switchButtonText: {
    fontSize: 12,
    fontWeight: '600',
    color: '#0066CC',
  },
});
