import React from 'react';
import {
  ActivityIndicator,
  Alert,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { useOfflineCache, UseOfflineCacheResult } from '@hooks/useOfflineCaching';
import { useAppStore } from '@store/appStore';

export interface OfflineCachingProps {
  showStats?: boolean;
  showClearButton?: boolean;
  onCacheCleared?: () => void;
  testID?: string;
}

interface CacheStats {
  size: number;
  entries: number;
  formattedSize: string;
}

/**
 * Offline Caching component
 * Displays cache statistics and provides cache management controls
 */
export const OfflineCaching: React.FC<OfflineCachingProps> = ({
  showStats = true,
  showClearButton = true,
  onCacheCleared,
  testID,
}) => {
  const { getCacheSize, clearCache } = useOfflineCache();
  const { isOnline } = useAppStore();
  const [stats, setStats] = React.useState<CacheStats | null>(null);
  const [isClearing, setIsClearing] = React.useState(false);

  const calculateStats = React.useCallback(() => {
    const size = getCacheSize();

    const formattedSize =
      size > 1024 * 1024
        ? `${(size / (1024 * 1024)).toFixed(2)} MB`
        : size > 1024
          ? `${(size / 1024).toFixed(2)} KB`
          : `${size} B`;

    setStats({
      size,
      entries: Math.floor(size / 256), // Rough estimate
      formattedSize,
    });
  }, [getCacheSize]);

  React.useEffect(() => {
    calculateStats();
    const interval = setInterval(calculateStats, 5000); // Update every 5 seconds
    return () => clearInterval(interval);
  }, [calculateStats]);

  const handleClearCache = React.useCallback(() => {
    Alert.alert('Clear Cache', 'Are you sure you want to clear all cached data?', [
      {
        text: 'Cancel',
        onPress: () => {},
        style: 'cancel',
      },
      {
        text: 'Clear',
        onPress: () => {
          setIsClearing(true);
          try {
            clearCache();
            calculateStats();
            Alert.alert('Success', 'Cache cleared successfully');
            onCacheCleared?.();
          } catch (error) {
            Alert.alert('Error', 'Failed to clear cache');
          } finally {
            setIsClearing(false);
          }
        },
        style: 'destructive',
      },
    ]);
  }, [clearCache, calculateStats, onCacheCleared]);

  return (
    <ScrollView
      style={styles.container}
      contentContainerStyle={styles.content}
      testID={testID}
      accessibilityLabel="Offline caching management">
      {/* Header */}
      <View style={styles.header}>
        <Text style={styles.title}>Offline Caching</Text>
        <Text style={styles.subtitle}>
          {isOnline ? 'Online - Using real-time data' : 'Offline - Using cached data'}
        </Text>
      </View>

      {/* Status Card */}
      <View
        style={[styles.statusCard, { backgroundColor: isOnline ? '#E8F5E9' : '#FFF3E0' }]}
        accessibilityLiveRegion="polite"
        accessibilityLabel={`${isOnline ? 'Online' : 'Offline'} status`}>
        <View style={styles.statusBadge}>
          <View style={[styles.statusDot, { backgroundColor: isOnline ? '#4CAF50' : '#FF9800' }]} />
          <Text style={styles.statusText}>{isOnline ? 'Online' : 'Offline'}</Text>
        </View>
        <Text style={styles.statusDescription}>
          {isOnline
            ? 'All data is synced in real-time'
            : 'Using cached data - will sync when connection returns'}
        </Text>
      </View>

      {/* Cache Statistics */}
      {showStats && stats && (
        <View style={styles.statsContainer}>
          <Text style={styles.sectionTitle}>Cache Statistics</Text>

          <View style={styles.statRow}>
            <Text style={styles.statLabel}>Cache Size</Text>
            <Text style={styles.statValue}>{stats.formattedSize}</Text>
          </View>

          <View style={styles.statRow}>
            <Text style={styles.statLabel}>Cached Items</Text>
            <Text style={styles.statValue}>~{stats.entries}</Text>
          </View>

          <View style={styles.statRow}>
            <Text style={styles.statLabel}>Status</Text>
            <Text style={[styles.statValue, { color: isOnline ? '#4CAF50' : '#FF9800' }]}>
              {stats.size > 0 ? 'Active' : 'Empty'}
            </Text>
          </View>
        </View>
      )}

      {/* Cache Management Section */}
      <View style={styles.managementSection}>
        <Text style={styles.sectionTitle}>Cache Management</Text>

        {stats && stats.size > 0 ? (
          <View style={styles.cacheInfo}>
            <Text style={styles.cacheInfoText}>
              💾 You have {stats.formattedSize} of cached data stored locally
            </Text>
            <Text style={styles.cacheInfoSubtext}>
              This data will be automatically synced when you're back online
            </Text>
          </View>
        ) : (
          <View style={styles.cacheInfo}>
            <Text style={styles.cacheInfoText}>📭 No cached data available</Text>
            <Text style={styles.cacheInfoSubtext}>Cache will be created when you go offline</Text>
          </View>
        )}

        {/* Clear Cache Button */}
        {showClearButton && (
          <Pressable
            style={({ pressed }) => [
              styles.clearButton,
              {
                backgroundColor: pressed ? '#C62828' : '#D32F2F',
                opacity: isClearing ? 0.6 : 1,
              },
            ]}
            onPress={handleClearCache}
            disabled={isClearing}
            accessibilityRole="button"
            accessibilityLabel="Clear cache">
            {isClearing ? (
              <>
                <ActivityIndicator color="#FFFFFF" size="small" />
                <Text style={styles.clearButtonText}>Clearing...</Text>
              </>
            ) : (
              <Text style={styles.clearButtonText}>Clear Cache</Text>
            )}
          </Pressable>
        )}
      </View>

      {/* Information Section */}
      <View style={styles.infoSection}>
        <Text style={styles.sectionTitle}>How Offline Caching Works</Text>

        <View style={styles.infoItem}>
          <Text style={styles.infoBullet}>•</Text>
          <Text style={styles.infoText}>Automatically caches data when you view it</Text>
        </View>

        <View style={styles.infoItem}>
          <Text style={styles.infoBullet}>•</Text>
          <Text style={styles.infoText}>Works offline using cached copies</Text>
        </View>

        <View style={styles.infoItem}>
          <Text style={styles.infoBullet}>•</Text>
          <Text style={styles.infoText}>Syncs data automatically when connection returns</Text>
        </View>

        <View style={styles.infoItem}>
          <Text style={styles.infoBullet}>•</Text>
          <Text style={styles.infoText}>Cache expires after 24 hours of last access</Text>
        </View>
      </View>

      {/* Platform Info */}
      <View style={styles.platformInfo}>
        <Text style={styles.platformLabel}>Platform</Text>
        <Text style={styles.platformValue}>{Platform.OS === 'ios' ? 'iOS' : 'Android'}</Text>
      </View>
    </ScrollView>
  );
};

/**
 * Minimal offline caching indicator component
 * Use this in headers or status bars
 */
export interface OfflineCachingIndicatorProps {
  showCacheSize?: boolean;
  testID?: string;
}

export const OfflineCachingIndicator: React.FC<OfflineCachingIndicatorProps> = ({
  showCacheSize = false,
  testID,
}) => {
  const { getCacheSize } = useOfflineCache();
  const { isOnline } = useAppStore();
  const [cacheSize, setCacheSize] = React.useState(0);

  React.useEffect(() => {
    const updateSize = () => setCacheSize(getCacheSize());
    updateSize();
    const interval = setInterval(updateSize, 10000);
    return () => clearInterval(interval);
  }, [getCacheSize]);

  const formatSize = (bytes: number) => {
    if (bytes > 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }
    if (bytes > 1024) {
      return `${(bytes / 1024).toFixed(1)} KB`;
    }
    return `${bytes} B`;
  };

  return (
    <View
      style={styles.indicatorContainer}
      testID={testID}
      accessibilityLabel={`Offline caching indicator - ${isOnline ? 'Online' : 'Offline'}`}>
      <View style={[styles.indicatorDot, { backgroundColor: isOnline ? '#4CAF50' : '#FF9800' }]} />
      <Text style={styles.indicatorText}>
        {isOnline ? 'Online' : 'Offline'}
        {showCacheSize && cacheSize > 0 && ` • ${formatSize(cacheSize)}`}
      </Text>
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#FAFAFA',
  },
  content: {
    paddingHorizontal: 16,
    paddingVertical: 20,
  },
  header: {
    marginBottom: 24,
  },
  title: {
    fontSize: 28,
    fontWeight: '700',
    marginBottom: 8,
    color: '#212121',
  },
  subtitle: {
    fontSize: 14,
    color: '#666666',
    fontWeight: '500',
  },
  statusCard: {
    borderRadius: 12,
    padding: 16,
    marginBottom: 24,
    borderWidth: 1,
    borderColor: 'rgba(0, 0, 0, 0.05)',
  },
  statusBadge: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: 8,
  },
  statusDot: {
    width: 12,
    height: 12,
    borderRadius: 6,
    marginRight: 8,
  },
  statusText: {
    fontSize: 16,
    fontWeight: '600',
    color: '#212121',
  },
  statusDescription: {
    fontSize: 13,
    color: '#666666',
    lineHeight: 20,
  },
  statsContainer: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    marginBottom: 24,
    borderWidth: 1,
    borderColor: '#E0E0E0',
  },
  sectionTitle: {
    fontSize: 16,
    fontWeight: '600',
    marginBottom: 16,
    color: '#212121',
  },
  statRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: '#E0E0E0',
  },
  statLabel: {
    fontSize: 14,
    color: '#666666',
    fontWeight: '500',
  },
  statValue: {
    fontSize: 14,
    fontWeight: '600',
    color: '#212121',
  },
  managementSection: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    marginBottom: 24,
    borderWidth: 1,
    borderColor: '#E0E0E0',
  },
  cacheInfo: {
    backgroundColor: '#F5F5F5',
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
  },
  cacheInfoText: {
    fontSize: 14,
    fontWeight: '500',
    color: '#212121',
    marginBottom: 4,
  },
  cacheInfoSubtext: {
    fontSize: 13,
    color: '#999999',
  },
  clearButton: {
    paddingVertical: 12,
    paddingHorizontal: 16,
    borderRadius: 8,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
  },
  clearButtonText: {
    color: '#FFFFFF',
    fontSize: 14,
    fontWeight: '600',
  },
  infoSection: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    marginBottom: 24,
    borderWidth: 1,
    borderColor: '#E0E0E0',
  },
  infoItem: {
    flexDirection: 'row',
    marginBottom: 12,
  },
  infoBullet: {
    fontSize: 16,
    color: '#0066CC',
    marginRight: 8,
    marginTop: -2,
  },
  infoText: {
    fontSize: 13,
    color: '#666666',
    flex: 1,
    lineHeight: 20,
  },
  platformInfo: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    borderWidth: 1,
    borderColor: '#E0E0E0',
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  platformLabel: {
    fontSize: 14,
    color: '#666666',
    fontWeight: '500',
  },
  platformValue: {
    fontSize: 14,
    fontWeight: '600',
    color: '#212121',
  },
  // Indicator styles
  indicatorContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 12,
    paddingVertical: 6,
    backgroundColor: '#F5F5F5',
    borderRadius: 6,
  },
  indicatorDot: {
    width: 8,
    height: 8,
    borderRadius: 4,
    marginRight: 6,
  },
  indicatorText: {
    fontSize: 12,
    fontWeight: '500',
    color: '#666666',
  },
});
