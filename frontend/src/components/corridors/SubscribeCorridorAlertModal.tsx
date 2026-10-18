"use client";

import React, { useState } from "react";
import { Bell, CheckCircle2, AlertCircle, X, Mail, Globe } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { corridorAlertsApi } from "@/lib/alerts-api";
import { logger } from "@/lib/logger";

interface SubscribeCorridorAlertModalProps {
  corridorKey: string;
  isOpen: boolean;
  onClose: () => void;
}

export function SubscribeCorridorAlertModal({
  corridorKey,
  isOpen,
  onClose,
}: SubscribeCorridorAlertModalProps) {
  const [notifyEmail, setNotifyEmail] = useState(true);
  const [notifyWebhook, setNotifyWebhook] = useState(false);
  const [notifyInApp, setNotifyInApp] = useState(true);
  const [email, setEmail] = useState("");
  const [webhookUrl, setWebhookUrl] = useState("");
  const [alertOnStatusChange, setAlertOnStatusChange] = useState(true);
  const [loading, setLoading] = useState(false);
  const [success, setSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (notifyEmail && !email.trim()) {
      setError("Please provide an email address for email notifications.");
      return;
    }
    if (notifyWebhook && !webhookUrl.trim()) {
      setError("Please provide a webhook URL for webhook delivery.");
      return;
    }
    if (!notifyEmail && !notifyWebhook && !notifyInApp) {
      setError("Please select at least one notification channel.");
      return;
    }

    setLoading(true);
    try {
      await corridorAlertsApi.createConfig({
        name: `Status Alerts for ${corridorKey}`,
        corridor_key: corridorKey,
        alert_on_status_change: alertOnStatusChange,
        email: notifyEmail ? email.trim() : undefined,
        webhook_url: notifyWebhook ? webhookUrl.trim() : undefined,
        notify_email: notifyEmail,
        notify_webhook: notifyWebhook,
        notify_in_app: notifyInApp,
        cooldown_seconds: 300,
      });

      setSuccess(true);
      setTimeout(() => {
        setSuccess(false);
        onClose();
      }, 1500);
    } catch (err) {
      logger.error("Failed to subscribe to corridor alerts:", err);
      setError(err instanceof Error ? err.message : "Failed to subscribe to corridor alerts");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="relative w-full max-w-lg bg-white dark:bg-slate-900 rounded-xl shadow-2xl border border-gray-200 dark:border-slate-800 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-100 dark:border-slate-800">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-blue-50 dark:bg-blue-950 text-blue-600 dark:text-blue-400 rounded-lg">
              <Bell className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                Subscribe to Corridor Alerts
              </h3>
              <p className="text-xs text-muted-foreground mt-0.5">
                Corridor: <span className="font-mono font-medium">{corridorKey}</span>
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-slate-800 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <form onSubmit={handleSubmit} className="p-6 space-y-5">
          {success ? (
            <div className="py-6 flex flex-col items-center justify-center text-center space-y-3">
              <CheckCircle2 className="w-12 h-12 text-green-500 animate-bounce" />
              <h4 className="text-base font-medium text-gray-900 dark:text-white">
                Subscribed Successfully!
              </h4>
              <p className="text-sm text-muted-foreground max-w-xs">
                You will receive alerts whenever the status of {corridorKey} changes.
              </p>
            </div>
          ) : (
            <>
              {error && (
                <div className="p-3 bg-red-50 dark:bg-red-950/50 border border-red-200 dark:border-red-900 text-red-700 dark:text-red-300 rounded-lg text-xs flex items-center gap-2">
                  <AlertCircle className="w-4 h-4 shrink-0" />
                  <span>{error}</span>
                </div>
              )}

              {/* Status Transition Trigger */}
              <div className="flex items-center justify-between p-3.5 bg-gray-50 dark:bg-slate-800/60 rounded-lg border border-gray-100 dark:border-slate-800">
                <div className="space-y-0.5">
                  <Label htmlFor="status-toggle" className="text-sm font-medium cursor-pointer">
                    Corridor Status Changes
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Notify on transitions between Healthy, Degraded, and Critical
                  </p>
                </div>
                <Switch
                  id="status-toggle"
                  checked={alertOnStatusChange}
                  onCheckedChange={setAlertOnStatusChange}
                />
              </div>

              {/* Channels */}
              <div className="space-y-3.5">
                <Label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                  Delivery Channels
                </Label>

                {/* Email Option */}
                <div className="space-y-2 border border-gray-100 dark:border-slate-800 rounded-lg p-3 bg-gray-50/50 dark:bg-slate-800/30">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Mail className="w-4 h-4 text-blue-500" />
                      <Label htmlFor="email-channel" className="text-sm font-medium cursor-pointer">
                        Email Notifications
                      </Label>
                    </div>
                    <Switch
                      id="email-channel"
                      checked={notifyEmail}
                      onCheckedChange={setNotifyEmail}
                    />
                  </div>
                  {notifyEmail && (
                    <div className="pt-2">
                      <Input
                        type="email"
                        placeholder="you@example.com"
                        value={email}
                        onChange={(e) => setEmail(e.target.value)}
                        className="text-sm h-9"
                        required={notifyEmail}
                      />
                    </div>
                  )}
                </div>

                {/* Webhook Option */}
                <div className="space-y-2 border border-gray-100 dark:border-slate-800 rounded-lg p-3 bg-gray-50/50 dark:bg-slate-800/30">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Globe className="w-4 h-4 text-purple-500" />
                      <Label htmlFor="webhook-channel" className="text-sm font-medium cursor-pointer">
                        Webhook Delivery
                      </Label>
                    </div>
                    <Switch
                      id="webhook-channel"
                      checked={notifyWebhook}
                      onCheckedChange={setNotifyWebhook}
                    />
                  </div>
                  {notifyWebhook && (
                    <div className="pt-2">
                      <Input
                        type="url"
                        placeholder="https://api.yourdomain.com/webhooks/status"
                        value={webhookUrl}
                        onChange={(e) => setWebhookUrl(e.target.value)}
                        className="text-sm h-9 font-mono text-xs"
                        required={notifyWebhook}
                      />
                    </div>
                  )}
                </div>

                {/* In-App Option */}
                <div className="flex items-center justify-between p-3 border border-gray-100 dark:border-slate-800 rounded-lg bg-gray-50/50 dark:bg-slate-800/30">
                  <div className="flex items-center gap-2">
                    <Bell className="w-4 h-4 text-amber-500" />
                    <Label htmlFor="inapp-channel" className="text-sm font-medium cursor-pointer">
                      In-App Notification Center
                    </Label>
                  </div>
                  <Switch
                    id="inapp-channel"
                    checked={notifyInApp}
                    onCheckedChange={setNotifyInApp}
                  />
                </div>
              </div>

              {/* Action Buttons */}
              <div className="flex items-center justify-end gap-3 pt-3 border-t border-gray-100 dark:border-slate-800">
                <Button
                  type="button"
                  variant="outline"
                  onClick={onClose}
                  disabled={loading}
                >
                  Cancel
                </Button>
                <Button
                  type="submit"
                  disabled={loading}
                  className="bg-blue-600 hover:bg-blue-700 text-white"
                >
                  {loading ? "Subscribing..." : "Subscribe"}
                </Button>
              </div>
            </>
          )}
        </form>
      </div>
    </div>
  );
}
