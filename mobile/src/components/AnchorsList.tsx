import React from 'react';
import {
  ActivityIndicator,
  FlatList,
  Platform,
  Pressable,
  RefreshControl,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import { NativeStackNavigationProp } from '@react-navigation/native-stack';

import { useAnchorsList, AnchorListItem } from '@hooks/useAnchorsList';
import { AnchorsStackParamList } from '@navigation/MainNavigator';

export interface AnchorsListProps {
  onAnchorPress?: (anchorId: string) => void;
}

type AnchorsListNavigationProp = NativeStackNavigationProp<
  AnchorsStackParamList,
  'AnchorsList'
>;

function truncateAddress(address: string): string {
  if (address.length <= 16) {
    return address;
  }
  return `${address.slice(0, 8)}...${address.slice(-8)}`;
}

function formatNumber(value: number): string {
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  }
  if (value >= 1_000) {
    return `${(value / 1_000).toFixed(1)}K`;
  }
  return value.toLocaleString();
}

function calculateUptime(anchor: AnchorListItem): number {
  if (anchor.total_transactions <= 0) {
    return 0;
  }
  return (anchor.successful_transactions / anchor.total_transactions) * 100;
}

function getStatusStyles(status: string) {
  switch (status.toLowerCase()) {
    case 'green':
    case 'active':
    case 'healthy':
      return {
        badge: styles.statusHealthy,
        text: styles.statusHealthyText,
        label: 'Healthy',
      };
    case 'yellow':
    case 'degraded':
    case 'warning':
      return {
        badge: styles.statusWarning,
        text: styles.statusWarningText,
        label: 'Degraded',
      };
    case 'red':
    case 'inactive':
    case 'critical':
      return {
        badge: styles.statusCritical,
        text: styles.statusCriticalText,
        label: 'Critical',
      };
    default:
      return {
        badge: styles.statusUnknown,
        text: styles.statusUnknownText,
        label: 'Unknown',
      };
  }
}

function getFeedbackBannerStyle(isFromCache: boolean) {
  if (!isFromCache) {
    return {
      container: styles.mockBanner,
      text: styles.mockBannerText,
    };
  }
  return {
    container: styles.feedbackBanner,
    text: styles.feedbackText,
  };
}

interface AnchorCardProps {
  anchor: AnchorListItem;
  onPress?: (anchorId: string) => void;
}

const AnchorCard: React.FC<AnchorCardProps> = ({ anchor, onPress }) => {
  const uptime = calculateUptime(anchor);
  const statusStyles = getStatusStyles(anchor.status);

  return (
    <Pressable
      style={({ pressed }) => [styles.card, pressed && onPress && styles.cardPressed]}
      onPress={onPress ? () => onPress(anchor.id) : undefined}
      accessibilityRole={onPress ? 'button' : 'text'}
      accessibilityLabel={`${anchor.name}, ${statusStyles.label}, reliability ${anchor.reliability_score.toFixed(1)} percent, uptime ${uptime.toFixed(1)} percent`}
    >
      <View style={styles.cardHeader}>
        <View style={styles.cardTitleBlock}>
          <Text style={styles.cardTitle}>{anchor.name}</Text>
          <Text style={styles.cardSubtitle}>{truncateAddress(anchor.stellar_account)}</Text>
        </View>
        <View style={[styles.statusBadge, statusStyles.badge]}>
          <Text style={[styles.statusBadgeText, statusStyles.text]}>
            {statusStyles.label}
          </Text>
        </View>
      </View>

      <View style={styles.metricsRow}>
        <View style={styles.metricBlock}>
          <Text style={styles.metricLabel}>Reliability</Text>
          <Text style={styles.metricValue}>{anchor.reliability_score.toFixed(1)}%</Text>
        </View>
        <View style={styles.metricBlock}>
          <Text style={styles.metricLabel}>Uptime</Text>
          <Text style={styles.metricValue}>{uptime.toFixed(1)}%</Text>
        </View>
        <View style={styles.metricBlock}>
          <Text style={styles.metricLabel}>Assets</Text>
          <Text style={styles.metricValue}>{anchor.asset_coverage}</Text>
        </View>
      </View>

      <Text style={styles.cardMeta}>
        {formatNumber(anchor.successful_transactions)} /{' '}
        {formatNumber(anchor.total_transactions)} transactions · Failure rate{' '}
        {anchor.failure_rate.toFixed(1)}%
      </Text>
    </Pressable>
  );
};

export const AnchorsList: React.FC<AnchorsListProps> = ({ onAnchorPress }) => {
  const navigation = useNavigation<AnchorsListNavigationProp>();
  const {
    items: anchors,
    total,
    loading,
    loadingMore,
    hasMore,
    error,
    warning,
    isFromCache,
    refresh,
    loadMore,
  } = useAnchorsList();

  const handleAnchorPress = (anchorId: string) => {
    if (onAnchorPress) {
      onAnchorPress(anchorId);
      return;
    }
    navigation.navigate('AnchorDetail', { anchorId });
  };

  if (loading && anchors.length === 0 && !error) {
    return (
      <SafeAreaView style={styles.container} edges={['bottom']}>
        <View
          style={styles.centerContent}
          accessible
          accessibilityRole="progressbar"
          accessibilityLabel="Loading anchors list"
        >
          <ActivityIndicator
            size="large"
            color={Platform.OS === 'ios' ? '#007AFF' : '#1976D2'}
          />
          <Text style={styles.loadingText}>Loading anchors...</Text>
        </View>
      </SafeAreaView>
    );
  }

  if (error && anchors.length === 0) {
    return (
      <SafeAreaView style={styles.container} edges={['bottom']}>
        <View style={styles.content}>
          <View
            style={styles.errorBanner}
            accessible
            accessibilityRole="alert"
            accessibilityLabel={error}
          >
            <Text style={styles.errorText}>{error}</Text>
            <Pressable
              onPress={() => { void refresh(); }}
              style={styles.retryButton}
              accessibilityRole="button"
              accessibilityLabel="Retry loading anchors list"
            >
              <Text style={styles.retryButtonText}>Retry</Text>
            </Pressable>
          </View>
        </View>
      </SafeAreaView>
    );
  }

  const feedbackBanner = warning ? getFeedbackBannerStyle(isFromCache) : null;

  return (
    <SafeAreaView style={styles.container} edges={['bottom']}>
      <FlatList
        data={anchors}
        keyExtractor={item => item.id}
        contentContainerStyle={styles.listContent}
        accessibilityLabel="Anchors list"
        refreshControl={
          <RefreshControl
            refreshing={loading}
            onRefresh={() => { void refresh(); }}
            tintColor={Platform.OS === 'ios' ? '#007AFF' : undefined}
            colors={Platform.OS === 'android' ? ['#1976D2'] : undefined}
          />
        }
        onEndReached={hasMore ? () => { void loadMore(); } : undefined}
        onEndReachedThreshold={0.3}
        ListFooterComponent={
          loadingMore ? (
            <View style={styles.loadMoreSpinner}>
              <ActivityIndicator
                size="small"
                color={Platform.OS === 'ios' ? '#007AFF' : '#1976D2'}
              />
            </View>
          ) : null
        }
        ListHeaderComponent={
          <>
            <View
              style={styles.header}
              accessible
              accessibilityRole="header"
              accessibilityLabel={`Anchors list, ${total} total anchors`}
            >
              <Text style={styles.title}>Anchors List</Text>
              <Text style={styles.subtitle}>{total} anchor{total === 1 ? '' : 's'}</Text>
            </View>

            {warning && feedbackBanner ? (
              <View
                style={feedbackBanner.container}
                accessible
                accessibilityRole="text"
                accessibilityLabel={warning}
              >
                <Text style={feedbackBanner.text}>{warning}</Text>
                {!isFromCache ? (
                  <Text style={feedbackBanner.text}>
                    Anchor metrics shown are for preview only and may not reflect live
                    performance.
                  </Text>
                ) : null}
              </View>
            ) : null}
          </>
        }
        renderItem={({ item }) => (
          <AnchorCard anchor={item} onPress={handleAnchorPress} />
        )}
        ListEmptyComponent={
          <View
            style={styles.empty}
            accessible
            accessibilityRole="text"
            accessibilityLabel="No anchors available"
          >
            <Text style={styles.emptyText}>No anchors available</Text>
          </View>
        }
      />
    </SafeAreaView>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: Platform.OS === 'ios' ? '#F2F2F7' : '#FAFAFA',
  },
  content: {
    flex: 1,
    paddingHorizontal: 16,
    paddingTop: 16,
  },
  listContent: {
    paddingHorizontal: 16,
    paddingBottom: 24,
  },
  centerContent: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 24,
  },
  loadingText: {
    marginTop: 12,
    fontSize: 16,
    color: '#525252',
  },
  header: {
    paddingTop: 8,
    paddingBottom: 16,
  },
  title: {
    fontSize: Platform.OS === 'ios' ? 28 : 24,
    fontWeight: '700',
    color: '#111827',
  },
  subtitle: {
    marginTop: 4,
    fontSize: 14,
    color: '#6B7280',
  },
  feedbackBanner: {
    backgroundColor: '#FEF3C7',
    borderColor: '#F59E0B',
    borderWidth: 1,
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
  },
  feedbackText: {
    color: '#92400E',
    fontSize: 14,
  },
  mockBanner: {
    backgroundColor: '#FFEDD5',
    borderColor: '#FB923C',
    borderWidth: 1,
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
    gap: 6,
  },
  mockBannerText: {
    color: '#9A3412',
    fontSize: 14,
  },
  card: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: '#E5E7EB',
    minHeight: 44,
    ...Platform.select({
      ios: {
        shadowColor: '#000',
        shadowOpacity: 0.05,
        shadowRadius: 4,
        shadowOffset: { width: 0, height: 2 },
      },
      android: {
        elevation: 2,
      },
    }),
  },
  cardPressed: {
    opacity: 0.85,
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    marginBottom: 12,
    gap: 12,
  },
  cardTitleBlock: {
    flex: 1,
  },
  cardTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: '#111827',
  },
  cardSubtitle: {
    marginTop: 4,
    fontSize: 12,
    color: '#6B7280',
    fontFamily: Platform.select({ ios: 'Menlo', android: 'monospace' }),
  },
  statusBadge: {
    borderRadius: 999,
    paddingHorizontal: 10,
    paddingVertical: 4,
  },
  statusBadgeText: {
    fontSize: 12,
    fontWeight: '600',
  },
  statusHealthy: {
    backgroundColor: '#DCFCE7',
  },
  statusHealthyText: {
    color: '#166534',
  },
  statusWarning: {
    backgroundColor: '#FEF9C3',
  },
  statusWarningText: {
    color: '#854D0E',
  },
  statusCritical: {
    backgroundColor: '#FEE2E2',
  },
  statusCriticalText: {
    color: '#991B1B',
  },
  statusUnknown: {
    backgroundColor: '#F3F4F6',
  },
  statusUnknownText: {
    color: '#374151',
  },
  metricsRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: 10,
    gap: 8,
  },
  metricBlock: {
    flex: 1,
  },
  metricLabel: {
    fontSize: 12,
    color: '#6B7280',
    marginBottom: 4,
  },
  metricValue: {
    fontSize: 18,
    fontWeight: '700',
    color: '#111827',
  },
  cardMeta: {
    fontSize: 12,
    color: '#6B7280',
    lineHeight: 18,
  },
  empty: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 40,
  },
  emptyText: {
    fontSize: 16,
    color: '#999',
  },
  errorBanner: {
    backgroundColor: '#FEE2E2',
    borderColor: '#FCA5A5',
    borderWidth: 1,
    borderRadius: 8,
    padding: 16,
  },
  errorText: {
    color: '#991B1B',
    fontSize: 15,
    marginBottom: 12,
  },
  retryButton: {
    alignSelf: 'flex-start',
    minHeight: 44,
    justifyContent: 'center',
    paddingHorizontal: 16,
    backgroundColor: '#DC2626',
    borderRadius: 8,
  },
  retryButtonText: {
    color: '#FFFFFF',
    fontWeight: '600',
  },
  loadMoreSpinner: {
    paddingVertical: 16,
    alignItems: 'center',
  },
});
