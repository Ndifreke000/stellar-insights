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
import { RouteProp, useNavigation, useRoute } from '@react-navigation/native';
import { NativeStackNavigationProp } from '@react-navigation/native-stack';

import { useCorridorDetail } from '@hooks/useCorridorDetail';
import {
  CorridorDataSource,
  CorridorDetailProps,
  CorridorMetrics,
} from '@app-types/corridor';
import { CorridorsStackParamList } from '@navigation/MainNavigator';

type CorridorDetailRouteProp = RouteProp<CorridorsStackParamList, 'CorridorDetail'>;
type CorridorDetailNavigationProp = NativeStackNavigationProp<
  CorridorsStackParamList,
  'CorridorDetail'
>;

function getHealthColor(score: number): string {
  if (score >= 90) {
    return '#16a34a';
  }
  if (score >= 75) {
    return '#ca8a04';
  }
  return '#dc2626';
}

function formatMillions(value: number): string {
  return `$${(value / 1_000_000).toFixed(2)}M`;
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

interface MetricCardProps {
  label: string;
  value: string;
  subtitle?: string;
  accessibilityLabel: string;
}

const MetricCard: React.FC<MetricCardProps> = ({
  label,
  value,
  subtitle,
  accessibilityLabel,
}) => (
  <View
    style={styles.metricCard}
    accessible
    accessibilityRole="summary"
    accessibilityLabel={accessibilityLabel}
  >
    <Text style={styles.metricLabel}>{label}</Text>
    <Text style={styles.metricValue}>{value}</Text>
    {subtitle ? <Text style={styles.metricSubtitle}>{subtitle}</Text> : null}
  </View>
);

interface RelatedCorridorCardProps {
  corridor: CorridorMetrics;
  onPress: (corridorId: string) => void;
}

const RelatedCorridorCard: React.FC<RelatedCorridorCardProps> = ({
  corridor,
  onPress,
}) => (
  <Pressable
    style={({ pressed }) => [
      styles.relatedCard,
      pressed && styles.relatedCardPressed,
    ]}
    onPress={() => onPress(corridor.id)}
    accessibilityRole="button"
    accessibilityLabel={`Open corridor ${corridor.source_asset} to ${corridor.destination_asset}`}
  >
    <View style={styles.relatedHeader}>
      <Text style={styles.relatedTitle}>
        {corridor.source_asset} → {corridor.destination_asset}
      </Text>
      <Text style={styles.relatedSuccess}>{corridor.success_rate.toFixed(1)}%</Text>
    </View>
    <Text style={styles.relatedMeta}>
      Health {corridor.health_score.toFixed(0)} · Liquidity{' '}
      {formatMillions(corridor.liquidity_depth_usd)}
    </Text>
  </Pressable>
);

export const CorridorDetail: React.FC<CorridorDetailProps> = ({
  corridorId: corridorIdProp,
  onNavigateToCorridor,
  onGoBack,
}) => {
  const navigation = useNavigation<CorridorDetailNavigationProp>();
  const route = useRoute<CorridorDetailRouteProp>();
  const corridorId = corridorIdProp ?? route.params?.corridorId ?? '';

  const { data, loading, error, warning, dataSource, refetch } =
    useCorridorDetail({ corridorId });

  const handleGoBack = () => {
    if (onGoBack) {
      onGoBack();
      return;
    }
    navigation.goBack();
  };

  const handleNavigateToCorridor = (nextCorridorId: string) => {
    if (onNavigateToCorridor) {
      onNavigateToCorridor(nextCorridorId);
      return;
    }
    navigation.push('CorridorDetail', { corridorId: nextCorridorId });
  };

  if (loading && !data) {
    return (
      <SafeAreaView style={styles.container} edges={['top', 'bottom']}>
        <View
          style={styles.centerContent}
          accessible
          accessibilityRole="progressbar"
          accessibilityLabel="Loading corridor detail"
        >
          <ActivityIndicator
            size="large"
            color={Platform.OS === 'ios' ? '#007AFF' : '#1976D2'}
          />
          <Text style={styles.loadingText}>Loading corridor detail...</Text>
        </View>
      </SafeAreaView>
    );
  }

  if (error || !data) {
    return (
      <SafeAreaView style={styles.container} edges={['top', 'bottom']}>
        <View style={styles.content}>
          <Pressable
            onPress={handleGoBack}
            style={styles.backButton}
            accessibilityRole="button"
            accessibilityLabel="Go back to corridors list"
          >
            <Text style={styles.backButtonText}>Back to Corridors</Text>
          </Pressable>
          <View
            style={styles.errorBanner}
            accessible
            accessibilityRole="alert"
            accessibilityLabel={error ?? 'Failed to load corridor data'}
          >
            <Text style={styles.errorText}>
              {error ?? 'Failed to load corridor data'}
            </Text>
            <Pressable
              onPress={() => {
                void refetch();
              }}
              style={styles.retryButton}
              accessibilityRole="button"
              accessibilityLabel="Retry loading corridor detail"
            >
              <Text style={styles.retryButtonText}>Retry</Text>
            </Pressable>
          </View>
        </View>
      </SafeAreaView>
    );
  }

  const { corridor } = data;
  const healthColor = getHealthColor(corridor.health_score);
  const feedbackBanner = warning ? getFeedbackBannerStyle(dataSource) : null;

  return (
    <SafeAreaView style={styles.container} edges={['top', 'bottom']}>
      <ScrollView
        contentContainerStyle={styles.content}
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
        <Pressable
          onPress={handleGoBack}
          style={styles.backButton}
          accessibilityRole="button"
          accessibilityLabel="Go back to corridors list"
        >
          <Text style={styles.backButtonText}>Back to Corridors</Text>
        </Pressable>

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
                Metrics shown are for preview only and may not reflect live corridor
                performance.
              </Text>
            ) : null}
          </View>
        ) : null}

        <View
          style={styles.header}
          accessible
          accessibilityRole="header"
          accessibilityLabel={`Corridor detail for ${corridor.source_asset} to ${corridor.destination_asset}`}
        >
          <View style={styles.headerTextBlock}>
            <Text style={styles.title}>
              {corridor.source_asset} → {corridor.destination_asset}
            </Text>
            <Text style={styles.subtitle}>Pair: {corridorId}</Text>
          </View>
          <View style={styles.healthBadge}>
            <Text style={[styles.healthScore, { color: healthColor }]}>
              {corridor.health_score.toFixed(1)}
            </Text>
            <Text style={styles.healthLabel}>Health Score</Text>
          </View>
        </View>

        <View style={styles.metricsGrid}>
          <MetricCard
            label="Success Rate"
            value={`${corridor.success_rate.toFixed(1)}%`}
            subtitle={`${corridor.successful_payments} of ${corridor.total_attempts}`}
            accessibilityLabel={`Success rate ${corridor.success_rate.toFixed(1)} percent`}
          />
          <MetricCard
            label="Avg Latency"
            value={`${corridor.average_latency_ms.toFixed(0)} ms`}
            subtitle={`Med ${corridor.median_latency_ms} ms · P99 ${corridor.p99_latency_ms} ms`}
            accessibilityLabel={`Average latency ${corridor.average_latency_ms.toFixed(0)} milliseconds`}
          />
          <MetricCard
            label="Liquidity Depth"
            value={formatMillions(corridor.liquidity_depth_usd)}
            subtitle={corridor.liquidity_trend}
            accessibilityLabel={`Liquidity depth ${formatMillions(corridor.liquidity_depth_usd)}`}
          />
          <MetricCard
            label="24h Volume"
            value={formatMillions(corridor.liquidity_volume_24h_usd)}
            subtitle={`Updated ${new Date(corridor.last_updated).toLocaleTimeString()}`}
            accessibilityLabel={`24 hour volume ${formatMillions(corridor.liquidity_volume_24h_usd)}`}
          />
        </View>

        <View style={styles.section}>
          <Text
            style={styles.sectionTitle}
            accessibilityRole="header"
            accessibilityLabel="Historical summary"
          >
            Historical Summary
          </Text>
          <Text style={styles.sectionBody}>
            Success samples: {data.historical_success_rate.length} · Latency
            buckets: {data.latency_distribution.length} · Liquidity points:{' '}
            {data.liquidity_trends.length}
          </Text>
        </View>

        {data.related_corridors && data.related_corridors.length > 0 ? (
          <View style={styles.section}>
            <Text
              style={styles.sectionTitle}
              accessibilityRole="header"
              accessibilityLabel="Related corridors"
            >
              Related Corridors
            </Text>
            {data.related_corridors.map(related => (
              <RelatedCorridorCard
                key={related.id}
                corridor={related}
                onPress={handleNavigateToCorridor}
              />
            ))}
          </View>
        ) : null}

        <Text
          style={styles.footer}
          accessibilityLabel={`Last updated ${new Date(corridor.last_updated).toLocaleString()}`}
        >
          Last updated: {new Date(corridor.last_updated).toLocaleString()}
        </Text>
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
  backButton: {
    minHeight: 44,
    justifyContent: 'center',
    marginBottom: 12,
  },
  backButtonText: {
    color: Platform.OS === 'ios' ? '#007AFF' : '#1976D2',
    fontSize: 16,
    fontWeight: '600',
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
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    marginBottom: 20,
    gap: 12,
  },
  headerTextBlock: {
    flex: 1,
  },
  title: {
    fontSize: Platform.OS === 'ios' ? 28 : 24,
    fontWeight: '700',
    color: '#111827',
  },
  subtitle: {
    marginTop: 4,
    fontSize: 13,
    color: '#6B7280',
    fontFamily: Platform.select({ ios: 'Menlo', android: 'monospace' }),
  },
  healthBadge: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    paddingHorizontal: 16,
    paddingVertical: 12,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: '#D1D5DB',
    alignItems: 'flex-end',
  },
  healthScore: {
    fontSize: 28,
    fontWeight: '700',
  },
  healthLabel: {
    fontSize: 11,
    color: '#6B7280',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  metricsGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
    marginBottom: 20,
  },
  metricCard: {
    width: Platform.OS === 'ios' ? '47%' : '48%',
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: '#E5E7EB',
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
  metricLabel: {
    fontSize: 13,
    color: '#6B7280',
    marginBottom: 8,
  },
  metricValue: {
    fontSize: 24,
    fontWeight: '700',
    color: '#111827',
  },
  metricSubtitle: {
    marginTop: 6,
    fontSize: 12,
    color: '#6B7280',
    textTransform: 'capitalize',
  },
  section: {
    marginBottom: 20,
  },
  sectionTitle: {
    fontSize: 20,
    fontWeight: '700',
    color: '#111827',
    marginBottom: 8,
  },
  sectionBody: {
    fontSize: 14,
    color: '#4B5563',
    lineHeight: 20,
  },
  relatedCard: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: '#E5E7EB',
  },
  relatedCardPressed: {
    opacity: 0.85,
  },
  relatedHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 6,
  },
  relatedTitle: {
    flex: 1,
    fontSize: 16,
    fontWeight: '600',
    color: '#111827',
  },
  relatedSuccess: {
    fontSize: 14,
    fontWeight: '700',
    color: '#16A34A',
  },
  relatedMeta: {
    fontSize: 13,
    color: '#6B7280',
  },
  footer: {
    fontSize: 13,
    color: '#6B7280',
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
