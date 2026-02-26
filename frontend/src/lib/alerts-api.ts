import { api } from "./api";

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
