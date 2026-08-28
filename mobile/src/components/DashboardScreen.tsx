import React from 'react';
import {
  ActivityIndicator,
  Platform,
  Pressable,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { useDashboardScreen } from '@hooks/useDashboardScreen';
import {
  CorridorPerformance,
  DashboardDataSource,
  DashboardStats,
} from '@app-types/dashboard';

const ACCENT_COLOR = Platform.OS === 'ios' ? '#007AFF' : '#1976D2';

export interface DashboardScreenProps {
  onCorridorPress?: (corridor: string) => void;
}

function formatCurrency(value: number): string {
  if (value >= 1_000_000) {
    return `$${(value / 1_000_000).toFixed(1)}M`;
  }
  if (value >= 1_000) {
    return `$${(value / 1_000).toFixed(1)}K`;
  }
  return `$${value.toLocaleString()}`;
}

function formatGrowth(value: number): string {
  const sign = value > 0 ? '+' : '';
  return `${sign}${value.toFixed(1)}%`;
}

function growthLabel(value: number): string {
  if (value > 0) {
    return `up ${Math.abs(value).toFixed(1)} percent`;
  }
  if (value < 0) {
    return `down ${Math.abs(value).toFixed(1)} percent`;
  }
  return 'no change';
}

function growthStyle(value: number) {
  if (value > 0) {
    return styles.growthPositive;
  }
  if (value < 0) {
    return styles.growthNegative;
  }
  return styles.growthNeutral;
}

function getHealthLabel(score: number): string {
  if (score >= 90) {
    return 'Healthy';
  }
  if (score >= 75) {
    return 'Fair';
  }
  return 'Critical';
}

function getFeedbackBannerStyle(dataSource: DashboardDataSource | null) {
  if (dataSource === 'mock') {
    return { container: styles.mockBanner, text: styles.mockBannerText };
  }
  return { container: styles.feedbackBanner, text: styles.feedbackText };
}

interface StatCardProps {
  label: string;
  value: string;
  growth: number;
  growthSuffix?: string;
}

const StatCard: React.FC<StatCardProps> = ({
  label,
  value,
  growth,
  growthSuffix,
}) => (
  <View
    style={styles.statCard}
    accessible
    accessibilityRole="text"
    accessibilityLabel={`${label}: ${value}, ${growthLabel(growth)}`}
  >
    <Text style={styles.statLabel}>{label}</Text>
    <Text style={styles.statValue}>{value}</Text>
    <Text style={[styles.statGrowth, growthStyle(growth)]}>
      {growthSuffix ? `${formatGrowth(growth)} ${growthSuffix}` : formatGrowth(growth)}
    </Text>
  </View>
);

interface CorridorRowProps {
  corridor: CorridorPerformance;
  onPress?: (corridor: string) => void;
}

const CorridorRow: React.FC<CorridorRowProps> = ({ corridor, onPress }) => {
  const healthLabel = getHealthLabel(corridor.health);

  return (
    <Pressable
      style={({ pressed }) => [styles.corridorRow, pressed && onPress && styles.rowPressed]}
      onPress={onPress ? () => onPress(corridor.corridor) : undefined}
      accessibilityRole={onPress ? 'button' : 'text'}
      accessibilityLabel={`${corridor.corridor}, ${healthLabel}, success rate ${corridor.success_rate.toFixed(1)} percent, volume ${formatCurrency(corridor.volume)}`}
    >
      <View style={styles.corridorInfo}>
        <Text style={styles.corridorName}>{corridor.corridor}</Text>
        <Text style={styles.corridorMeta}>
          {formatCurrency(corridor.volume)} · {healthLabel}
        </Text>
      </View>
      <Text style={styles.corridorRate}>{corridor.success_rate.toFixed(1)}%</Text>
    </Pressable>
  );
};

function renderStats(stats: DashboardStats) {
  return (
    <View style={styles.statsGrid}>
      <StatCard
        label="Volume (24h)"
        value={formatCurrency(stats.volume_24h)}
        growth={stats.volume_growth}
      />
      <StatCard
        label="Success Rate"
        value={`${stats.avg_success_rate.toFixed(1)}%`}
        growth={stats.success_rate_growth}
      />
      <StatCard
        label="Active Corridors"
        value={`${stats.active_corridors}`}
        growth={stats.corridors_growth}
      />
    </View>
  );
}

export const DashboardScreen: React.FC<DashboardScreenProps> = ({
  onCorridorPress,
}) => {
  const {
    stats,
    corridorPerformance,
    loading,
    error,
    warning,
    dataSource,
    refetch,
  } = useDashboardScreen();

  if (loading && !stats && !error) {
    return (
      <SafeAreaView style={styles.container} edges={['bottom']}>
        <View
          style={styles.centerContent}
          accessible
          accessibilityRole="progressbar"
          accessibilityLabel="Loading dashboard"
        >
          <ActivityIndicator size="large" color={ACCENT_COLOR} />
          <Text style={styles.loadingText}>Loading dashboard...</Text>
        </View>
      </SafeAreaView>
    );
  }

  if (error && !stats) {
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
              accessibilityLabel="Retry loading dashboard"
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
      <ScrollView
        contentContainerStyle={styles.listContent}
        accessibilityLabel="Dashboard"
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
      >
        <View
          style={styles.header}
          accessible
          accessibilityRole="header"
          accessibilityLabel="Dashboard"
        >
          <Text style={styles.title}>Dashboard</Text>
          <Text style={styles.subtitle}>Network payment analytics</Text>
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
                Dashboard metrics shown are for preview only and may not reflect
                live performance.
              </Text>
            ) : null}
          </View>
        ) : null}

        {stats ? renderStats(stats) : null}

        <View
          style={styles.section}
          accessible
          accessibilityRole="header"
          accessibilityLabel="Top corridors"
        >
          <Text style={styles.sectionTitle}>Top Corridors</Text>
        </View>

        {corridorPerformance.length > 0 ? (
          corridorPerformance.map(corridor => (
            <CorridorRow
              key={corridor.corridor}
              corridor={corridor}
              onPress={onCorridorPress}
            />
          ))
        ) : (
          <View
            style={styles.empty}
            accessible
            accessibilityRole="text"
            accessibilityLabel="No corridor data available"
          >
            <Text style={styles.emptyText}>No corridor data available</Text>
          </View>
        )}
      </ScrollView>
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
  statsGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
    marginBottom: 8,
  },
  statCard: {
    flexGrow: 1,
    flexBasis: '100%',
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
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
  statLabel: {
    fontSize: 14,
    color: '#6B7280',
    marginBottom: 8,
  },
  statValue: {
    fontSize: 28,
    fontWeight: '700',
    color: '#111827',
  },
  statGrowth: {
    marginTop: 4,
    fontSize: 13,
    fontWeight: '600',
  },
  growthPositive: {
    color: '#15803D',
  },
  growthNegative: {
    color: '#B91C1C',
  },
  growthNeutral: {
    color: '#6B7280',
  },
  section: {
    paddingTop: 16,
    paddingBottom: 8,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: '700',
    color: '#111827',
  },
  corridorRow: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
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
  rowPressed: {
    opacity: 0.85,
  },
  corridorInfo: {
    flex: 1,
    paddingRight: 12,
  },
  corridorName: {
    fontSize: 16,
    fontWeight: '600',
    color: '#111827',
  },
  corridorMeta: {
    marginTop: 4,
    fontSize: 13,
    color: '#6B7280',
  },
  corridorRate: {
    fontSize: 16,
    fontWeight: '700',
    color: ACCENT_COLOR,
  },
  errorBanner: {
    backgroundColor: '#FEE2E2',
    borderColor: '#EF4444',
    borderWidth: 1,
    borderRadius: 8,
    padding: 16,
  },
  errorText: {
    color: '#991B1B',
    fontSize: 14,
    marginBottom: 12,
  },
  retryButton: {
    alignSelf: 'flex-start',
    backgroundColor: ACCENT_COLOR,
    borderRadius: 8,
    paddingHorizontal: 16,
    paddingVertical: 8,
    minHeight: 44,
    justifyContent: 'center',
  },
  retryButtonText: {
    color: '#FFFFFF',
    fontSize: 14,
    fontWeight: '600',
  },
  empty: {
    padding: 24,
    alignItems: 'center',
  },
  emptyText: {
    fontSize: 14,
    color: '#6B7280',
  },
});
