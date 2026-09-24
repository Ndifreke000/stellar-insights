"use client";

import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  corridorAlertsApi,
  type CreateCorridorAlertConfigRequest,
  type CorridorAlertConfig,
} from "@/lib/alerts-api";
import { logger } from "@/lib/logger";

const configSchema = z.object({
  name: z.string().min(1, "Name is required").max(128),
  corridor_key: z.string().max(256).optional(),
  success_rate_threshold: z.number().min(0).max(1).optional(),
  latency_threshold_ms: z.number().min(0).optional(),
  liquidity_threshold_usd: z.number().min(0).optional(),
  success_rate_drop_pct: z.number().min(0.1).max(100).optional(),
  latency_increase_pct: z.number().min(0.1).max(1000).optional(),
  liquidity_drop_pct: z.number().min(0.1).max(100).optional(),
  cooldown_seconds: z.number().min(0).max(86400).optional(),
  notify_email: z.boolean().optional(),
  notify_webhook: z.boolean().optional(),
  notify_in_app: z.boolean().optional(),
  notify_slack: z.boolean().optional(),
  notify_telegram: z.boolean().optional(),
});

type ConfigFormData = z.infer<typeof configSchema>;

interface CorridorAlertConfigFormProps {
  existingConfig?: CorridorAlertConfig;
  onSaved: () => void;
  onCancel: () => void;
}

export function CorridorAlertConfigForm({
  existingConfig,
  onSaved,
  onCancel,
}: CorridorAlertConfigFormProps) {
  const [saving, setSaving] = useState(false);

  const {
    register,
    handleSubmit,
    watch,
    setValue,
    formState: { errors },
  } = useForm<ConfigFormData>({
    resolver: zodResolver(configSchema),
    defaultValues: {
      name: existingConfig?.name ?? "",
      corridor_key: existingConfig?.corridor_key ?? "",
      success_rate_threshold: existingConfig?.success_rate_threshold ?? undefined,
      latency_threshold_ms: existingConfig?.latency_threshold_ms ?? undefined,
      liquidity_threshold_usd: existingConfig?.liquidity_threshold_usd ?? undefined,
      success_rate_drop_pct: existingConfig?.success_rate_drop_pct ?? 10,
      latency_increase_pct: existingConfig?.latency_increase_pct ?? 50,
      liquidity_drop_pct: existingConfig?.liquidity_drop_pct ?? 30,
      cooldown_seconds: existingConfig?.cooldown_seconds ?? 300,
      notify_email: existingConfig?.notify_email ?? false,
      notify_webhook: existingConfig?.notify_webhook ?? false,
      notify_in_app: existingConfig?.notify_in_app ?? true,
      notify_slack: existingConfig?.notify_slack ?? false,
      notify_telegram: existingConfig?.notify_telegram ?? false,
    },
  });

  const notifyEmail = watch("notify_email");
  const notifyWebhook = watch("notify_webhook");
  const notifyInApp = watch("notify_in_app");
  const notifySlack = watch("notify_slack");
  const notifyTelegram = watch("notify_telegram");

  const onSubmit = async (data: ConfigFormData) => {
    setSaving(true);
    try {
      const payload: CreateCorridorAlertConfigRequest = {
        name: data.name,
        corridor_key: data.corridor_key || undefined,
        success_rate_threshold: data.success_rate_threshold,
        latency_threshold_ms: data.latency_threshold_ms,
        liquidity_threshold_usd: data.liquidity_threshold_usd,
        success_rate_drop_pct: data.success_rate_drop_pct,
        latency_increase_pct: data.latency_increase_pct,
        liquidity_drop_pct: data.liquidity_drop_pct,
        cooldown_seconds: data.cooldown_seconds,
        notify_email: data.notify_email,
        notify_webhook: data.notify_webhook,
        notify_in_app: data.notify_in_app,
        notify_slack: data.notify_slack,
        notify_telegram: data.notify_telegram,
      };

      if (existingConfig) {
        await corridorAlertsApi.updateConfig(existingConfig.id, payload);
      } else {
        await corridorAlertsApi.createConfig(payload);
      }
      onSaved();
    } catch (err) {
      logger.error("Failed to save corridor alert config:", err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>{existingConfig ? "Edit" : "Create"} Alert Config</CardTitle>
          <CardDescription>
            Configure thresholds and notification channels for corridor performance monitoring.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="name">Alert Name</Label>
            <Input id="name" {...register("name")} placeholder="e.g. USDC/EUR Critical Alerts" />
            {errors.name && (
              <p className="text-sm text-red-500">{errors.name.message}</p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="corridor_key">Corridor Key (optional)</Label>
            <Input
              id="corridor_key"
              {...register("corridor_key")}
              placeholder="Leave empty to monitor all corridors"
            />
          </div>

          <div className="border-t pt-4">
            <h4 className="text-sm font-medium mb-3">Absolute Thresholds</h4>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="space-y-2">
                <Label htmlFor="success_rate_threshold">Min Success Rate (0-1)</Label>
                <Input
                  id="success_rate_threshold"
                  type="number"
                  step="0.01"
                  {...register("success_rate_threshold", { valueAsNumber: true })}
                  placeholder="e.g. 0.9"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="latency_threshold_ms">Max Latency (ms)</Label>
                <Input
                  id="latency_threshold_ms"
                  type="number"
                  {...register("latency_threshold_ms", { valueAsNumber: true })}
                  placeholder="e.g. 500"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="liquidity_threshold_usd">Min Liquidity ($)</Label>
                <Input
                  id="liquidity_threshold_usd"
                  type="number"
                  {...register("liquidity_threshold_usd", { valueAsNumber: true })}
                  placeholder="e.g. 10000"
                />
              </div>
            </div>
          </div>

          <div className="border-t pt-4">
            <h4 className="text-sm font-medium mb-3">Relative Thresholds (% change)</h4>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="space-y-2">
                <Label htmlFor="success_rate_drop_pct">Success Rate Drop %</Label>
                <Input
                  id="success_rate_drop_pct"
                  type="number"
                  step="0.1"
                  {...register("success_rate_drop_pct", { valueAsNumber: true })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="latency_increase_pct">Latency Increase %</Label>
                <Input
                  id="latency_increase_pct"
                  type="number"
                  step="0.1"
                  {...register("latency_increase_pct", { valueAsNumber: true })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="liquidity_drop_pct">Liquidity Drop %</Label>
                <Input
                  id="liquidity_drop_pct"
                  type="number"
                  step="0.1"
                  {...register("liquidity_drop_pct", { valueAsNumber: true })}
                />
              </div>
            </div>
          </div>

          <div className="border-t pt-4">
            <h4 className="text-sm font-medium mb-3">Cooldown</h4>
            <div className="space-y-2">
              <Label htmlFor="cooldown_seconds">Cooldown (seconds)</Label>
              <Input
                id="cooldown_seconds"
                type="number"
                {...register("cooldown_seconds", { valueAsNumber: true })}
              />
            </div>
          </div>

          <div className="border-t pt-4">
            <h4 className="text-sm font-medium mb-3">Notification Channels</h4>
            <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
              {[
                { key: "notify_in_app" as const, label: "In-App", value: notifyInApp },
                { key: "notify_email" as const, label: "Email", value: notifyEmail },
                { key: "notify_webhook" as const, label: "Webhook", value: notifyWebhook },
                { key: "notify_slack" as const, label: "Slack", value: notifySlack },
                { key: "notify_telegram" as const, label: "Telegram", value: notifyTelegram },
              ].map((ch) => (
                <div key={ch.key} className="flex items-center space-x-2">
                  <Switch
                    checked={ch.value}
                    onCheckedChange={(v) => setValue(ch.key, v)}
                  />
                  <Label className="text-sm">{ch.label}</Label>
                </div>
              ))}
            </div>
          </div>
        </CardContent>
        <CardFooter className="flex justify-end gap-2">
          <Button type="button" variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button type="submit" disabled={saving}>
            {saving ? "Saving..." : existingConfig ? "Update" : "Create"}
          </Button>
        </CardFooter>
      </Card>
    </form>
  );
}
