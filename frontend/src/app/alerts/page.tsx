"use client";

import React, { useEffect, useState } from "react";
import {
    AlertRule,
    AlertHistory,
    alertsApi,
    CreateAlertRuleRequest,
    UpdateAlertRuleRequest,
} from "../../lib/alerts-api";
import { AlertRuleForm } from "../../components/AlertRuleForm";
import { Trash2, Edit2, Play, Square, Bell, BellOff, XCircle, Clock } from "lucide-react";

export default function AlertsPage() {
    const [rules, setRules] = useState<AlertRule[]>([]);
    const [history, setHistory] = useState<AlertHistory[]>([]);
    const [isFormOpen, setIsFormOpen] = useState(false);
    const [editingRule, setEditingRule] = useState<AlertRule | undefined>();
    const [loading, setLoading] = useState(true);

    // Load data
    const loadData = async () => {
        setLoading(true);
        try {
            const [fetchedRules, fetchedHistory] = await Promise.all([
                alertsApi.getRules(),
                alertsApi.getHistory(),
            ]);
            setRules(fetchedRules);
            setHistory(fetchedHistory);
        } catch (err) {
            console.error("Failed to fetch alerts data", err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadData();
    }, []);

    const handleCreateRule = async (data: CreateAlertRuleRequest | UpdateAlertRuleRequest) => {
        try {
            await alertsApi.createRule(data as CreateAlertRuleRequest);
            setIsFormOpen(false);
            loadData();
        } catch (err) {
            console.error("Failed to create rule", err);
        }
    };

    const handleUpdateRule = async (data: CreateAlertRuleRequest | UpdateAlertRuleRequest) => {
        if (!editingRule) return;
        try {
            await alertsApi.updateRule(editingRule.id, data as UpdateAlertRuleRequest);
            setEditingRule(undefined);
            setIsFormOpen(false);
            loadData();
        } catch (err) {
            console.error("Failed to update rule", err);
        }
    };

    const handleDeleteRule = async (id: string) => {
        if (!confirm("Are you sure you want to delete this rule?")) return;
        try {
            await alertsApi.deleteRule(id);
            loadData();
        } catch (err) {
            console.error("Failed to delete rule", err);
        }
    };

    const handleToggleActive = async (rule: AlertRule) => {
        try {
            await alertsApi.updateRule(rule.id, { is_active: !rule.is_active });
            loadData();
        } catch (err) {
            console.error("Failed to toggle rule", err);
        }
    };

    const handleDismissHistory = async (id: string) => {
        try {
            await alertsApi.dismissHistory(id);
            loadData();
        } catch (err) {
            console.error("Failed to dismiss alert", err);
        }
    };

    const handleSnooze = async (ruleId: string) => {
        try {
            // Snooze for 1 hour
            const snoozedUntil = new Date(Date.now() + 60 * 60 * 1000).toISOString();
            await alertsApi.snoozeRuleFromHistory(ruleId, { snoozed_until: snoozedUntil });
            loadData();
        } catch (err) {
            console.error("Failed to snooze rule", err);
        }
    };

    return (
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
            <div className="flex justify-between items-center mb-8">
                <div>
                    <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Alerts Overview</h1>
                    <p className="mt-1 text-sm text-muted-foreground dark:text-muted-foreground">
                        Manage your custom threshold alerts and review triggered events.
                    </p>
                </div>
                {!isFormOpen && (
                    <button
                        onClick={() => setIsFormOpen(true)}
                        className="inline-flex items-center px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 font-medium shadow-sm transition-colors"
                    >
                        Create Alert Rule
                    </button>
                )}
            </div>

            {isFormOpen && (
                <div className="mb-10 bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                    <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-6">
                        {editingRule ? "Edit Alert Rule" : "Create New Alert Rule"}
                    </h2>
                    <AlertRuleForm
                        initialData={editingRule}
                        onSubmit={editingRule ? handleUpdateRule : handleCreateRule}
                        onCancel={() => {
                            setIsFormOpen(false);
                            setEditingRule(undefined);
                        }}
                    />
                </div>
            )}

            <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
                {/* Left Column: Rules Management */}
                <div className="lg:col-span-2 space-y-6">
                    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                        <div className="px-6 py-5 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                            <h3 className="text-lg font-bold text-gray-900 dark:text-white">Active Rules</h3>
                        </div>

                        {loading ? (
                            <div className="p-8 justify-center flex items-center text-muted-foreground">Loading rules...</div>
                        ) : rules.length === 0 ? (
                            <div className="p-8 text-center text-muted-foreground">
                                You have not created any alert rules yet.
                            </div>
                        ) : (
                            <ul className="divide-y divide-gray-200 dark:divide-gray-700">
                                {rules.map((rule) => (
                                    <li key={rule.id} className="p-6 transition-colors hover:bg-gray-50 dark:hover:bg-gray-800/50">
                                        <div className="flex items-center justify-between">
                                            <div className="flex-1">
                                                <div className="flex items-center">
                                                    {rule.is_active ? (
                                                        <Bell className="h-5 w-5 text-blue-500 mr-3" />
                                                    ) : (
                                                        <BellOff className="h-5 w-5 text-muted-foreground mr-3" />
                                                    )}
                                                    <h4 className="text-lg font-semibold text-gray-900 dark:text-white">
                                                        {rule.metric_type.replace(/_/g, " ")} {rule.condition} {rule.threshold}
                                                    </h4>
                                                    {rule.corridor_id && (
                                                        <span className="ml-3 inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300">
                                                            {rule.corridor_id}
                                                        </span>
                                                    )}
                                                </div>
                                                <div className="mt-2 flex items-center text-sm text-muted-foreground dark:text-muted-foreground space-x-4">
                                                    <span>
                                                        Channels:{" "}
                                                        {[
                                                            rule.notify_in_app && "In-App",
                                                            rule.notify_email && "Email",
                                                            rule.notify_webhook && "Webhook"
                                                        ].filter(Boolean).join(", ")}
                                                    </span>
                                                    {rule.snoozed_until && new Date(rule.snoozed_until) > new Date() && (
                                                        <span className="flex items-center text-amber-600 dark:text-amber-500">
                                                            <Clock className="w-4 h-4 mr-1" />
                                                            Snoozed until {new Date(rule.snoozed_until).toLocaleTimeString()}
                                                        </span>
                                                    )}
                                                </div>
                                            </div>

                                            <div className="flex items-center space-x-2 ml-4">
                                                <button
                                                    onClick={() => handleToggleActive(rule)}
                                                    title={rule.is_active ? "Pause Rule" : "Activate Rule"}
                                                    className={`p-2 rounded-md ${rule.is_active
                                                            ? "text-rose-600 hover:bg-rose-50 dark:hover:bg-rose-900/30"
                                                            : "text-emerald-600 hover:bg-emerald-50 dark:hover:bg-emerald-900/30"
                                                        }`}
                                                >
                                                    {rule.is_active ? <Square className="w-4 h-4" /> : <Play className="w-4 h-4" />}
                                                </button>
                                                <button
                                                    onClick={() => {
                                                        setEditingRule(rule);
                                                        setIsFormOpen(true);
                                                        window.scrollTo({ top: 0, behavior: "smooth" });
                                                    }}
                                                    className="p-2 text-muted-foreground hover:text-blue-600 hover:bg-blue-50 rounded-md dark:hover:bg-blue-900/30"
                                                >
                                                    <Edit2 className="w-4 h-4" />
                                                </button>
                                                <button
                                                    onClick={() => handleDeleteRule(rule.id)}
                                                    className="p-2 text-muted-foreground hover:text-red-600 hover:bg-red-50 rounded-md dark:hover:bg-red-900/30"
                                                >
                                                    <Trash2 className="w-4 h-4" />
                                                </button>
                                            </div>
                                        </div>
                                    </li>
                                ))}
                            </ul>
                        )}
                    </div>
                </div>

                {/* Right Column: Alert History Log */}
                <div className="lg:col-span-1">
                    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden h-full flex flex-col">
                        <div className="px-6 py-5 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                            <h3 className="text-lg font-bold text-gray-900 dark:text-white">Recent Alerts</h3>
                        </div>

                        <div className="flex-1 overflow-y-auto max-h-[600px] p-4">
                            {loading ? (
                                <div className="text-center py-8 text-muted-foreground">Loading history...</div>
                            ) : history.filter(h => !h.is_dismissed).length === 0 ? (
                                <div className="text-center py-8 text-muted-foreground flex flex-col items-center">
                                    <BellOff className="w-12 h-12 text-gray-300 mb-3" />
                                    <p>No recent alerts.</p>
                                </div>
                            ) : (
                                <div className="space-y-4">
                                    {history.filter(h => !h.is_dismissed).map((alert) => (
                                        <div
                                            key={alert.id}
                                            className={`p-4 rounded-lg border ${alert.is_read
                                                    ? "bg-gray-50 border-gray-200 dark:bg-gray-800/50 dark:border-gray-700"
                                                    : "bg-blue-50 border-blue-200 dark:bg-blue-900/20 dark:border-blue-800"
                                                }`}
                                        >
                                            <div className="flex justify-between items-start mb-2">
                                                <span className="text-xs font-medium text-muted-foreground dark:text-muted-foreground">
                                                    {new Date(alert.triggered_at).toLocaleString()}
                                                </span>
                                                <div className="flex space-x-1">
                                                    <button
                                                        onClick={() => handleSnooze(alert.rule_id)}
                                                        title="Snooze this rule for 1 hour"
                                                        className="p-1 text-muted-foreground hover:text-amber-600 rounded"
                                                    >
                                                        <Clock className="w-4 h-4" />
                                                    </button>
                                                    <button
                                                        onClick={() => handleDismissHistory(alert.id)}
                                                        title="Dismiss Alert"
                                                        className="p-1 text-muted-foreground hover:text-gray-700 rounded"
                                                    >
                                                        <XCircle className="w-4 h-4" />
                                                    </button>
                                                </div>
                                            </div>
                                            <p className="text-sm font-medium text-gray-900 dark:text-gray-100">
                                                {alert.message}
                                            </p>
                                            {alert.corridor_id && (
                                                <p className="text-xs text-muted-foreground mt-2 font-mono">
                                                    {alert.corridor_id}
                                                </p>
                                            )}
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
