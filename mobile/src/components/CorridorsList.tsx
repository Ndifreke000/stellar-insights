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

import { useCorridorsList } from '@hooks/useCorridorsList';
import { CorridorDataSource, CorridorMetrics } from '@types/corridor';
import { CorridorsStackParamList } from '@navigation/MainNavigator';

export interface CorridorsListProps {
  onCorridorPress?: (corridorId: string) => void;
}

type CorridorsListNavigationProp = NativeStackNavigationProp<
  CorridorsStackParamList,
  'CorridorsList'
>;

function formatMillions(value: number): string {
  if (value >= 1_000_000) {
    return `$${(value / 1_000_000).toFixed(1)}M`;
  }
  if (value >= 1_000) {
    return `$${(value / 1_000).toFixed(1)}K`;
  }
  return `$${value.toLocaleString()}`;
}

function getHealthStyles(score: number) {
  if (score >= 90) {
    return {
      badge: styles.statusHealthy,
      text: styles.statusHealthyText,
      label: 'Healthy',
    };
  }
  if (score >= 75) {
    return {
      badge: styles.statusWarning,
      text: styles.statusWarningText,
      label: 'Fair',
    };
  }
  return {
    badge: styles.statusCritical,
    text: styles.statusCriticalText,
    label: 'Critical',
  };
}

function getFeedbackBannerStyle(dataSource: CorridorDataSource | null) {
  if (dataSource === 'mock') {
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

interface CorridorCardProps {
  corridor: CorridorMetrics;
  onPress?: (corridorId: string) => void;
}

const CorridorCard: React.FC<CorridorCardProps> = ({ corridor, onPress }) => {
  const healthStyles = getHealthStyles(corridor.health_score);

  return (
    <Pressable
      style={({ pressed }) => [styles.card, pressed && onPress && styles.cardPressed]}
      onPress={onPress ? () => onPress(corridor.id) : undefined}
      accessibilityRole={onPress ? 'button' : 'text'}
      accessibilityLabel={`${corridor.source_asset} to ${corridor.destination_asset}, ${healthStyles.label}, success rate ${corridor.success_rate.toFixed(1)} percent, health score ${corridor.health_score.toFixed(0)}`}
    >
      <View style={styles.cardHeader}>
        <View style={styles.cardTitleBlock}>
          <Text style={styles.cardTitle}>
            {corridor.source_asset} → {corridor.destination_asset}
          </Text>
          <Text style={styles.cardSubtitle}>{corridor.id}</Text>
        </View>
        <View style={[styles.statusBadge, healthStyles.badge]}>
          <Text style={[styles.statusBadgeText, healthStyles.text]}>
            {healthStyles.label}
          </Text>
        </View>
      </View>

      <View style={styles.metricsRow}>
        <View style={styles.metricBlock}>
          <Text style={styles.metricLabel}>Success Rate</Text>
          <Text style={styles.metricValue}>{corridor.success_rate.toFixed(1)}%</Text>
        </View>
        <View style={styles.metricBlock}>
          <Text style={styles.metricLabel}>Health</Text>
          <Text style={styles.metricValue}>{corridor.health_score.toFixed(0)}</Text>
        </View>
        <View style={styles.metricBlock}>
          <Text style={styles.metricLabel}>Liquidity</Text>
          <Text style={styles.metricValue}>
            {formatMillions(corridor.liquidity_depth_usd)}
          </Text>
        </View>
      </View>

      <Text style={styles.cardMeta}>
        {corridor.successful_payments.toLocaleString()} /{' '}
        {corridor.total_attempts.toLocaleString()} payments · Avg latency{' '}
        {corridor.average_latency_ms.toFixed(0)} ms
      </Text>
    </Pressable>
  );
};

export const CorridorsList: React.FC<CorridorsListProps> = ({ onCorridorPress }) => {
  const navigation = useNavigation<CorridorsListNavigationProp>();
  const {
    corridors,
    total,
    loading,
    error,
    warning,
    dataSource,
    refetch,
  } = useCorridorsList();

  const handleCorridorPress = (corridorId: string) => {
    if (onCorridorPress) {
      onCorridorPress(corridorId);
      return;
    }
    navigation.navigate('CorridorDetail', { corridorId });
  };

  if (loading && corridors.length === 0 && !error) {
    return (
      <SafeAreaView style={styles.container} edges={['bottom']}>
        <View
          style={styles.centerContent}
          accessible
          accessibilityRole="progressbar"
          accessibilityLabel="Loading corridors list"
        >
          <ActivityIndicator
            size="large"
            color={Platform.OS === 'ios' ? '#007AFF' : '#1976D2'}
          />
          <Text style={styles.loadingText}>Loading corridors...</Text>
        </View>
      </SafeAreaView>
    );
  }

  if (error && corridors.length === 0) {
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
              onPress={() => {
                void refetch();
              }}
              style={styles.retryButton}
              accessibilityRole="button"
              accessibilityLabel="Retry loading corridors list"
            >
              <Text style={styles.retryButtonText}>Retry</Text>
            </Pressable>
          </View>
        </View>
      </SafeAreaView>
    );
  }

  const feedbackBanner = warning ? getFeedbackBannerStyle(dataSource) : null;

  return (
    <SafeAreaView style={styles.container} edges={['bottom']}>
      <FlatList
        data={corridors}
        keyExtractor={item => item.id}
        contentContainerStyle={styles.listContent}
        accessibilityLabel="Corridors list"
        refreshControl={
          <RefreshControl
            refreshing={loading}
            onRefresh={() => {
              void refetch();
            }}
            tintColor={Platform.OS === 'ios' ? '#007AFF' : undefined}
            colors={Platform.OS === 'android' ? ['#1976D2'] : undefined}
          />
        }
        ListHeaderComponent={
          <>
            <View
              style={styles.header}
              accessible
              accessibilityRole="header"
              accessibilityLabel={`Corridors list, ${total} total corridors`}
            >
              <Text style={styles.title}>Corridors List</Text>
              <Text style={styles.subtitle}>
                {total} corridor{total === 1 ? '' : 's'}
              </Text>
            </View>

            {warning && feedbackBanner ? (
              <View
                style={feedbackBanner.container}
                accessible
                accessibilityRole="text"
                accessibilityLabel={warning}
              >
                <Text style={feedbackBanner.text}>{warning}</Text>
                {dataSource === 'mock' ? (
                  <Text style={feedbackBanner.text}>
                    Corridor metrics shown are for preview only and may not reflect
                    live performance.
                  </Text>
                ) : null}
              </View>
            ) : null}
          </>
        }
        renderItem={({ item }) => (
          <CorridorCard corridor={item} onPress={handleCorridorPress} />
        )}
        ListEmptyComponent={
          <View
            style={styles.empty}
            accessible
            accessibilityRole="text"
            accessibilityLabel="No corridors available"
          >
            <Text style={styles.emptyText}>No corridors available</Text>
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
});
