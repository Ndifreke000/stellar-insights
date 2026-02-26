import React, { useState } from "react";
import { CreateAlertRuleRequest, UpdateAlertRuleRequest, AlertRule } from "../lib/alerts-api";

interface AlertRuleFormProps {
    initialData?: AlertRule;
    onSubmit: (data: CreateAlertRuleRequest | UpdateAlertRuleRequest) => void;
    onCancel: () => void;
    isLoading?: boolean;
}

export function AlertRuleForm({ initialData, onSubmit, onCancel, isLoading }: AlertRuleFormProps) {
    const [metricType, setMetricType] = useState(initialData?.metric_type || "success_rate");
    const [condition, setCondition] = useState<"above" | "below" | "equals">(
        initialData?.condition || "below"
    );
    const [threshold, setThreshold] = useState<string>(
        initialData ? initialData.threshold.toString() : "90"
    );
    const [corridorId, setCorridorId] = useState(initialData?.corridor_id || "");
    const [notifyEmail, setNotifyEmail] = useState(initialData?.notify_email || false);
    const [notifyWebhook, setNotifyWebhook] = useState(initialData?.notify_webhook || false);
    const [notifyInApp, setNotifyInApp] = useState(initialData?.notify_in_app ?? true);

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        const data: CreateAlertRuleRequest = {
            metric_type: metricType,
            condition,
            threshold: parseFloat(threshold),
            notify_email: notifyEmail,
            notify_webhook: notifyWebhook,
            notify_in_app: notifyInApp,
            ...(corridorId ? { corridor_id: corridorId } : {}),
        };
        onSubmit(data);
    };

    return (
        <form onSubmit={handleSubmit} className="space-y-6">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        Metric
                    </label>
                    <select
                        value={metricType}
                        onChange={(e) => setMetricType(e.target.value)}
                        className="w-full px-4 py-2 bg-gray-50 border border-gray-300 rounded-lg text-gray-900 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    >
                        <option value="success_rate">Success Rate (%)</option>
                        <option value="latency">Latency (ms)</option>
                        <option value="liquidity">Liquidity Depth (USD)</option>
                        <option value="volume">Volume (USD)</option>
                    </select>
                </div>

                <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        Corridor (Optional)
                    </label>
                    <input
                        type="text"
                        placeholder="e.g. USDC-PHP"
                        value={corridorId}
                        onChange={(e) => setCorridorId(e.target.value)}
                        className="w-full px-4 py-2 bg-gray-50 border border-gray-300 rounded-lg text-gray-900 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    />
                </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        Condition
                    </label>
                    <select
                        value={condition}
                        onChange={(e) => setCondition(e.target.value as "above" | "below" | "equals")}
                        className="w-full px-4 py-2 bg-gray-50 border border-gray-300 rounded-lg text-gray-900 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    >
                        <option value="below">Drops Below</option>
                        <option value="above">Rises Above</option>
                        <option value="equals">Equals</option>
                    </select>
                </div>

                <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                        Threshold Value
                    </label>
                    <input
                        type="number"
                        step="any"
                        required
                        value={threshold}
                        onChange={(e) => setThreshold(e.target.value)}
                        className="w-full px-4 py-2 bg-gray-50 border border-gray-300 rounded-lg text-gray-900 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                    />
                </div>
            </div>

            <div className="space-y-3">
                <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300">Notification Channels</h4>

                <div className="flex items-center">
                    <input
                        id="notify-in-app"
                        type="checkbox"
                        checked={notifyInApp}
                        onChange={(e) => setNotifyInApp(e.target.checked)}
                        className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 dark:bg-gray-700 dark:border-gray-600"
                    />
                    <label htmlFor="notify-in-app" className="ml-2 text-sm text-gray-900 dark:text-gray-300">
                        In-App Notification
                    </label>
                </div>

                <div className="flex items-center">
                    <input
                        id="notify-email"
                        type="checkbox"
                        checked={notifyEmail}
                        onChange={(e) => setNotifyEmail(e.target.checked)}
                        className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 dark:bg-gray-700 dark:border-gray-600"
                    />
                    <label htmlFor="notify-email" className="ml-2 text-sm text-gray-900 dark:text-gray-300">
                        Email Notification
                    </label>
                </div>

                <div className="flex items-center">
                    <input
                        id="notify-webhook"
                        type="checkbox"
                        checked={notifyWebhook}
                        onChange={(e) => setNotifyWebhook(e.target.checked)}
                        className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 dark:bg-gray-700 dark:border-gray-600"
                    />
                    <label htmlFor="notify-webhook" className="ml-2 text-sm text-gray-900 dark:text-gray-300">
                        Webhook Delivery
                    </label>
                </div>
            </div>

            <div className="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-700">
                <button
                    type="button"
                    onClick={onCancel}
                    disabled={isLoading}
                    className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 dark:bg-gray-800 dark:text-gray-300 dark:border-gray-600 dark:hover:bg-gray-700"
                >
                    Cancel
                </button>
                <button
                    type="submit"
                    disabled={isLoading}
                    className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
                >
                    {isLoading ? "Saving..." : initialData ? "Update Rule" : "Create Rule"}
                </button>
            </div>
        </form>
    );
}
