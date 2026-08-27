import { api } from "./api/api";

export interface AlertRule {
    id: string;
    user_id: string;
    corridor_id?: string;
    metric_type: string;
    condition: "above" | "below" | "equals";
    threshold: number;
    notify_email: boolean;
    notify_webhook: boolean;
    notify_in_app: boolean;
    is_active: boolean;
    snoozed_until?: string;
    created_at: string;
    updated_at: string;
}

export interface AlertHistory {
    id: string;
    rule_id: string;
    user_id: string;
    corridor_id?: string;
    metric_type: string;
    trigger_value: number;
    threshold_value: number;
    condition: string;
    message: string;
    is_read: boolean;
    is_dismissed: boolean;
    triggered_at: string;
}

export interface CreateAlertRuleRequest {
    corridor_id?: string;
    metric_type: string;
    condition: "above" | "below" | "equals";
    threshold: number;
    notify_email: boolean;
    notify_webhook: boolean;
    notify_in_app: boolean;
}

export interface UpdateAlertRuleRequest {
    corridor_id?: string;
    metric_type?: string;
    condition?: "above" | "below" | "equals";
    threshold?: number;
    notify_email?: boolean;
    notify_webhook?: boolean;
    notify_in_app?: boolean;
    is_active?: boolean;
}

export interface SnoozeAlertRequest {
    snoozed_until: string;
}

export const alertsApi = {
    // Rule Operations
    getRules: () => api.get<AlertRule[]>("/alerts/rules"),
    createRule: (data: CreateAlertRuleRequest) => api.post<AlertRule>("/alerts/rules", data),
    updateRule: (id: string, data: UpdateAlertRuleRequest) => api.put<AlertRule>(`/alerts/rules/${id}`, data),
    deleteRule: (id: string) => api.delete<void>(`/alerts/rules/${id}`),

    // History Operations
    getHistory: () => api.get<AlertHistory[]>("/alerts/history"),
    markHistoryRead: (id: string) => api.post<void>(`/alerts/history/${id}/read`),
    dismissHistory: (id: string) => api.post<void>(`/alerts/history/${id}/dismiss`),
    snoozeRuleFromHistory: (ruleId: string, data: SnoozeAlertRequest) => api.post<AlertRule>(`/alerts/history/${ruleId}/snooze`, data),
};

// ---- Corridor Performance Alert Types ----

export interface CorridorPerformanceSnapshot {
    id: string;
    corridor_key: string;
    source_asset_code: string;
    source_asset_issuer: string;
    destination_asset_code: string;
    destination_asset_issuer: string;
    success_rate: number;
    avg_settlement_latency_ms: number;
    liquidity_depth_usd: number;
    volume_usd: number;
    total_transactions: number;
    successful_transactions: number;
    failed_transactions: number;
    snapshot_time: string;
    created_at: string;
}

export interface CorridorAlertConfig {
    id: string;
    user_id: string;
    corridor_key?: string;
    name: string;
    success_rate_threshold?: number;
    latency_threshold_ms?: number;
    liquidity_threshold_usd?: number;
    success_rate_drop_pct: number;
    latency_increase_pct: number;
    liquidity_drop_pct: number;
    cooldown_seconds: number;
    notify_email: boolean;
    notify_webhook: boolean;
    notify_in_app: boolean;
    notify_slack: boolean;
    notify_telegram: boolean;
    is_active: boolean;
    last_triggered_at?: string;
    created_at: string;
    updated_at: string;
}

export interface CorridorAlertEvent {
    id: string;
    config_id: string;
    user_id: string;
    corridor_key: string;
    alert_type: string;
    severity: "warning" | "critical";
    message: string;
    old_value?: number;
    new_value?: number;
    threshold_value?: number;
    acknowledged: boolean;
    acknowledged_at?: string;
    triggered_at: string;
    created_at: string;
}

export interface CorridorPerformanceSummary {
    corridor_key: string;
    current_success_rate: number;
    previous_success_rate?: number;
    current_latency_ms: number;
    previous_latency_ms?: number;
    current_liquidity_usd: number;
    previous_liquidity_usd?: number;
    success_rate_trend: number;
    latency_trend: number;
    liquidity_trend: number;
    alert_count_24h: number;
    status: "healthy" | "warning" | "critical";
}

export interface CorridorPerformanceTimeline {
    corridor_key: string;
    snapshots: CorridorPerformanceSnapshot[];
    alerts: CorridorAlertEvent[];
}

export interface CreateCorridorAlertConfigRequest {
    corridor_key?: string;
    name: string;
    success_rate_threshold?: number;
    latency_threshold_ms?: number;
    liquidity_threshold_usd?: number;
    success_rate_drop_pct?: number;
    latency_increase_pct?: number;
    liquidity_drop_pct?: number;
    cooldown_seconds?: number;
    notify_email?: boolean;
    notify_webhook?: boolean;
    notify_in_app?: boolean;
    notify_slack?: boolean;
    notify_telegram?: boolean;
}

export interface UpdateCorridorAlertConfigRequest {
    name?: string;
    success_rate_threshold?: number;
    latency_threshold_ms?: number;
    liquidity_threshold_usd?: number;
    success_rate_drop_pct?: number;
    latency_increase_pct?: number;
    liquidity_drop_pct?: number;
    cooldown_seconds?: number;
    notify_email?: boolean;
    notify_webhook?: boolean;
    notify_in_app?: boolean;
    notify_slack?: boolean;
    notify_telegram?: boolean;
    is_active?: boolean;
}

export interface CorridorPerformanceAlert {
    config_id: string;
    corridor_key: string;
    alert_type: string;
    severity: string;
    message: string;
    old_value?: number;
    new_value?: number;
    threshold_value?: number;
    timestamp: string;
}

// ---- Corridor Performance Alert API ----

export const corridorAlertsApi = {
    // Config Operations
    getConfigs: () => api.get<CorridorAlertConfig[]>("/corridor-alerts/configs"),
    getConfig: (id: string) => api.get<CorridorAlertConfig>(`/corridor-alerts/configs/${id}`),
    createConfig: (data: CreateCorridorAlertConfigRequest) =>
        api.post<CorridorAlertConfig>("/corridor-alerts/configs", data),
    updateConfig: (id: string, data: UpdateCorridorAlertConfigRequest) =>
        api.put<CorridorAlertConfig>(`/corridor-alerts/configs/${id}`, data),
    deleteConfig: (id: string) => api.delete<void>(`/corridor-alerts/configs/${id}`),

    // Snapshot Operations
    getLatestSnapshots: () => api.get<CorridorPerformanceSnapshot[]>("/corridor-alerts/snapshots"),
    getCorridorSnapshots: (corridorKey: string) =>
        api.get<CorridorPerformanceSnapshot[]>(`/corridor-alerts/snapshots/${encodeURIComponent(corridorKey)}`),
    getCorridorTimeline: (corridorKey: string) =>
        api.get<CorridorPerformanceTimeline>(`/corridor-alerts/snapshots/${encodeURIComponent(corridorKey)}/timeline`),

    // Summary Operations
    getPerformanceSummary: () => api.get<CorridorPerformanceSummary[]>("/corridor-alerts/summary"),
    getCorridorSummary: (corridorKey: string) =>
        api.get<CorridorPerformanceSummary>(`/corridor-alerts/summary/${encodeURIComponent(corridorKey)}`),

    // Event Operations
    getEvents: () => api.get<CorridorAlertEvent[]>("/corridor-alerts/events"),
    getCorridorEvents: (corridorKey: string) =>
        api.get<CorridorAlertEvent[]>(`/corridor-alerts/events/${encodeURIComponent(corridorKey)}`),
    acknowledgeEvent: (id: string) =>
        api.post<void>(`/corridor-alerts/events/${id}/acknowledge`),
    getUnreadCount: () => api.get<{ count: number }>("/corridor-alerts/unread-count"),
};
