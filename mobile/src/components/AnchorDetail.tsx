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

import { useAnchorDetail } from '@hooks/useAnchorDetail';
import {
  AnchorDataSource,
  AnchorDetailProps,
  FailedCorridorRef,
  FailureReason,
  IssuedAsset,
} from '@app-types/anchor';
import { AnchorsStackParamList } from '@navigation/MainNavigator';

type AnchorDetailRouteProp = RouteProp<AnchorsStackParamList, 'AnchorDetail'>;
type AnchorDetailNavigationProp = NativeStackNavigationProp<
  AnchorsStackParamList,
  'AnchorDetail'
>;

function truncateAddress(address: string): string {
  if (address.length <= 16) {
    return address;
  }
  return `${address.slice(0, 8)}...${address.slice(-8)}`;
}

function formatVolume(value: number): string {
  if (value >= 1_000_000) {
    return `$${(value / 1_000_000).toFixed(2)}M`;
  }
  if (value >= 1_000) {
    return `$${(value / 1_000).toFixed(1)}K`;
  }
  return `$${value.toLocaleString()}`;
}

function getReliabilityColor(score: number): string {
  if (score >= 90) {
    return '#16a34a';
  }
  if (score >= 75) {
    return '#ca8a04';
  }
  return '#dc2626';
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

function getFeedbackBannerStyle(dataSource: AnchorDataSource | null) {
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

function calculateUptime(
  successful: number,
  total: number,
): number {
  if (total <= 0) {
    return 0;
  }
  return (successful / total) * 100;
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

interface IssuedAssetCardProps {
  asset: IssuedAsset;
}

const IssuedAssetCard: React.FC<IssuedAssetCardProps> = ({ asset }) => (
  <View
    style={styles.assetCard}
    accessible
    accessibilityRole="text"
    accessibilityLabel={`${asset.asset_code} asset, success rate ${asset.success_rate.toFixed(1)} percent, volume ${formatVolume(asset.volume_24h_usd)}`}
  >
    <View style={styles.assetHeader}>
      <Text style={styles.assetTitle}>{asset.asset_code}</Text>
      <Text style={styles.assetSuccess}>{asset.success_rate.toFixed(1)}%</Text>
    </View>
    <Text style={styles.assetMeta}>
      Volume {formatVolume(asset.volume_24h_usd)} · {asset.total_transactions.toLocaleString()}{' '}
      transactions
    </Text>
    <Text style={styles.assetIssuer}>{truncateAddress(asset.issuer)}</Text>
  </View>
);

interface FailedCorridorCardProps {
  corridor: FailedCorridorRef;
  onPress: (corridorId: string) => void;
}

const FailedCorridorCard: React.FC<FailedCorridorCardProps> = ({ corridor, onPress }) => (
  <Pressable
    style={({ pressed }) => [styles.relatedCard, pressed && styles.relatedCardPressed]}
    onPress={() => onPress(corridor.corridor_id)}
    accessibilityRole="button"
    accessibilityLabel={`Open failed corridor ${corridor.corridor_id}`}
  >
    <Text style={styles.relatedTitle}>{corridor.corridor_id}</Text>
    <Text style={styles.relatedMeta}>
      {new Date(corridor.timestamp).toLocaleString()}
    </Text>
  </Pressable>
);

export const AnchorDetail: React.FC<AnchorDetailProps> = ({
  anchorId: anchorIdProp,
  onNavigateToCorridor,
  onGoBack,
}) => {
  const navigation = useNavigation<AnchorDetailNavigationProp>();
  const route = useRoute<AnchorDetailRouteProp>();
  const anchorId = anchorIdProp ?? route.params?.anchorId ?? '';

  const { data, loading, error, warning, dataSource, refetch } =
    useAnchorDetail({ anchorId });

  const handleGoBack = () => {
    if (onGoBack) {
      onGoBack();
      return;
    }
    navigation.goBack();
  };

  const handleNavigateToCorridor = (corridorId: string) => {
    if (onNavigateToCorridor) {
      onNavigateToCorridor(corridorId);
      return;
    }
    const parent = navigation.getParent();
    if (parent) {
      parent.navigate('Corridors', {
        screen: 'CorridorDetail',
        params: { corridorId },
      });
    }
  };

  if (loading && !data) {
    return (
      <SafeAreaView style={styles.container} edges={['top', 'bottom']}>
        <View
          style={styles.centerContent}
          accessible
          accessibilityRole="progressbar"
          accessibilityLabel="Loading anchor detail"
        >
          <ActivityIndicator
            size="large"
            color={Platform.OS === 'ios' ? '#007AFF' : '#1976D2'}
          />
          <Text style={styles.loadingText}>Loading anchor detail...</Text>
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
            accessibilityLabel="Go back to anchors list"
          >
            <Text style={styles.backButtonText}>Back to Anchors</Text>
          </Pressable>
          <View
            style={styles.errorBanner}
            accessible
            accessibilityRole="alert"
            accessibilityLabel={error ?? 'Failed to load anchor data'}
          >
            <Text style={styles.errorText}>
              {error ?? 'Failed to load anchor data'}
            </Text>
            <Pressable
              onPress={() => {
                void refetch();
              }}
              style={styles.retryButton}
              accessibilityRole="button"
              accessibilityLabel="Retry loading anchor detail"
            >
              <Text style={styles.retryButtonText}>Retry</Text>
            </Pressable>
          </View>
        </View>
      </SafeAreaView>
    );
  }

  const { anchor } = data;
  const uptime = calculateUptime(
    anchor.successful_transactions,
    anchor.total_transactions,
  );
  const reliabilityColor = getReliabilityColor(anchor.reliability_score);
  const statusStyles = getStatusStyles(anchor.status);
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
          accessibilityLabel="Go back to anchors list"
        >
          <Text style={styles.backButtonText}>Back to Anchors</Text>
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
                Metrics shown are for preview only and may not reflect live anchor
                performance.
              </Text>
            ) : null}
          </View>
        ) : null}

        <View
          style={styles.header}
          accessible
          accessibilityRole="header"
          accessibilityLabel={`Anchor detail for ${anchor.name}`}
        >
          <View style={styles.headerTextBlock}>
            <Text style={styles.title}>{anchor.name}</Text>
            <Text style={styles.subtitle}>{truncateAddress(anchor.stellar_account)}</Text>
          </View>
          <View style={styles.reliabilityBadge}>
            <Text style={[styles.reliabilityScore, { color: reliabilityColor }]}>
              {anchor.reliability_score.toFixed(1)}
            </Text>
            <Text style={styles.reliabilityLabel}>Reliability</Text>
          </View>
        </View>

        <View style={[styles.statusBadge, statusStyles.badge]}>
          <Text style={[styles.statusBadgeText, statusStyles.text]}>
            {statusStyles.label}
          </Text>
        </View>

        <View style={styles.metricsGrid}>
          <MetricCard
            label="Uptime"
            value={`${uptime.toFixed(1)}%`}
            subtitle={`${anchor.successful_transactions.toLocaleString()} of ${anchor.total_transactions.toLocaleString()}`}
            accessibilityLabel={`Uptime ${uptime.toFixed(1)} percent`}
          />
          <MetricCard
            label="Failure Rate"
            value={`${anchor.failure_rate.toFixed(1)}%`}
            subtitle={`${anchor.failed_transactions.toLocaleString()} failed`}
            accessibilityLabel={`Failure rate ${anchor.failure_rate.toFixed(1)} percent`}
          />
          <MetricCard
            label="Asset Coverage"
            value={String(anchor.asset_coverage)}
            subtitle={`${data.issued_assets.length} issued assets`}
            accessibilityLabel={`Asset coverage ${anchor.asset_coverage}`}
          />
          <MetricCard
            label="Reliability Trend"
            value={`${data.reliability_history.length} days`}
            subtitle="Historical samples"
            accessibilityLabel={`Reliability history with ${data.reliability_history.length} data points`}
          />
        </View>

        {data.issued_assets.length > 0 ? (
          <View style={styles.section}>
            <Text
              style={styles.sectionTitle}
              accessibilityRole="header"
              accessibilityLabel="Issued assets"
            >
              Issued Assets
            </Text>
            {data.issued_assets.map(asset => (
              <IssuedAssetCard key={`${asset.asset_code}-${asset.issuer}`} asset={asset} />
            ))}
          </View>
        ) : null}

        {data.top_failure_reasons && data.top_failure_reasons.length > 0 ? (
          <View style={styles.section}>
            <Text
              style={styles.sectionTitle}
              accessibilityRole="header"
              accessibilityLabel="Top failure reasons"
            >
              Top Failure Reasons
            </Text>
            {data.top_failure_reasons.map((item: FailureReason) => (
              <View
                key={item.reason}
                style={styles.reasonRow}
                accessible
                accessibilityLabel={`${item.reason}, ${item.count} occurrences`}
              >
                <Text style={styles.reasonText}>{item.reason}</Text>
                <Text style={styles.reasonCount}>{item.count}</Text>
              </View>
            ))}
          </View>
        ) : null}

        {data.recent_failed_corridors && data.recent_failed_corridors.length > 0 ? (
          <View style={styles.section}>
            <Text
              style={styles.sectionTitle}
              accessibilityRole="header"
              accessibilityLabel="Recent failed corridors"
            >
              Recent Failed Corridors
            </Text>
            {data.recent_failed_corridors.map(corridor => (
              <FailedCorridorCard
                key={`${corridor.corridor_id}-${corridor.timestamp}`}
                corridor={corridor}
                onPress={handleNavigateToCorridor}
              />
            ))}
          </View>
        ) : null}
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
    marginBottom: 12,
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
  reliabilityBadge: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    paddingHorizontal: 16,
    paddingVertical: 12,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: '#D1D5DB',
    alignItems: 'flex-end',
  },
  reliabilityScore: {
    fontSize: 28,
    fontWeight: '700',
  },
  reliabilityLabel: {
    fontSize: 11,
    color: '#6B7280',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  statusBadge: {
    alignSelf: 'flex-start',
    borderRadius: 999,
    paddingHorizontal: 10,
    paddingVertical: 4,
    marginBottom: 16,
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
  assetCard: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: '#E5E7EB',
  },
  assetHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 6,
  },
  assetTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: '#111827',
  },
  assetSuccess: {
    fontSize: 14,
    fontWeight: '700',
    color: '#16A34A',
  },
  assetMeta: {
    fontSize: 13,
    color: '#6B7280',
    marginBottom: 4,
  },
  assetIssuer: {
    fontSize: 12,
    color: '#6B7280',
    fontFamily: Platform.select({ ios: 'Menlo', android: 'monospace' }),
  },
  reasonRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    backgroundColor: '#FFFFFF',
    borderRadius: 8,
    padding: 12,
    marginBottom: 8,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: '#E5E7EB',
  },
  reasonText: {
    flex: 1,
    fontSize: 14,
    color: '#374151',
    marginRight: 8,
  },
  reasonCount: {
    fontSize: 14,
    fontWeight: '700',
    color: '#111827',
  },
  relatedCard: {
    backgroundColor: '#FFFFFF',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    borderWidth: StyleSheet.hairlineWidth,
    borderColor: '#E5E7EB',
    minHeight: 44,
  },
  relatedCardPressed: {
    opacity: 0.85,
  },
  relatedTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: '#111827',
    marginBottom: 4,
  },
  relatedMeta: {
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
