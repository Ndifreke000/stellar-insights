"use client";

import { useEffect, useState, useCallback } from "react";
import {
  Activity,
  AlertTriangle,
  Bell,
  CheckCircle,
  Settings,
  TrendingDown,
  TrendingUp,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  corridorAlertsApi,
  type CorridorPerformanceSummary,
  type CorridorAlertConfig,
} from "@/lib/alerts-api";
import { useCorridorPerformanceAlerts } from "@/hooks/useCorridorPerformanceAlerts";
import { CorridorPerformanceChart } from "./CorridorPerformanceChart";
import { CorridorAlertConfigForm } from "./CorridorAlertConfigForm";
import { CorridorAlertEventsList } from "./CorridorAlertEventsList";
import { logger } from "@/lib/logger";

function StatusDot({ status }: { status: string }) {
  const color =
    status === "critical"
      ? "bg-red-500"
      : status === "warning"
        ? "bg-yellow-500"
        : "bg-green-500";
  return <span className={`inline-block h-2 w-2 rounded-full ${color}`} />;
}

function TrendIndicator({ value, invert }: { value: number; invert?: boolean }) {
  const isBad = invert ? value > 0 : value < 0;
  if (Math.abs(value) < 0.01) return <span className="text-muted-foreground">--</span>;
  return (
    <span className={`inline-flex items-center text-xs ${isBad ? "text-red-500" : "text-green-500"}`}>
      {value > 0 ? <TrendingUp className="h-3 w-3 mr-0.5" /> : <TrendingDown className="h-3 w-3 mr-0.5" />}
      {Math.abs(value).toFixed(1)}%
    </span>
  );
}

export function CorridorPerformanceDashboard() {
  const [summaries, setSummaries] = useState<CorridorPerformanceSummary[]>([]);
  const [configs, setConfigs] = useState<CorridorAlertConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState("overview");
  const [editingConfig, setEditingConfig] = useState<CorridorAlertConfig | undefined>();
  const [showNewConfig, setShowNewConfig] = useState(false);
  const [selectedCorridor, setSelectedCorridor] = useState<string | null>(null);

  const { alerts: realtimeAlerts } = useCorridorPerformanceAlerts({
    onAlert: (alert) => {
      logger.debug("Real-time corridor alert:", alert);
      fetchSummaries();
    },
  });

  const fetchSummaries = useCallback(async () => {
    try {
      const [summaryData, configData] = await Promise.all([
        corridorAlertsApi.getPerformanceSummary(),
        corridorAlertsApi.getConfigs(),
      ]);
      setSummaries(summaryData);
      setConfigs(configData);
    } catch (err) {
      logger.error("Failed to fetch corridor data:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchSummaries();
  }, [fetchSummaries]);

  const handleConfigSaved = () => {
    setShowNewConfig(false);
    setEditingConfig(undefined);
    fetchSummaries();
  };

  const handleDeleteConfig = async (id: string) => {
    try {
      await corridorAlertsApi.deleteConfig(id);
      fetchSummaries();
    } catch (err) {
      logger.error("Failed to delete config:", err);
    }
  };

  if (loading) {
    return (
      <div className="space-y-4">
        <div className="h-8 w-64 bg-slate-200 dark:bg-slate-800 rounded animate-pulse" />
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-32 bg-slate-200 dark:bg-slate-800 rounded-xl animate-pulse" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Corridor Performance Alerts</h1>
          <p className="text-muted-foreground">
            Monitor corridor metrics and manage alert thresholds
          </p>
        </div>
        <Button onClick={() => { setShowNewConfig(true); setEditingConfig(undefined); }}>
          <Bell className="h-4 w-4 mr-2" />
          New Alert Config
        </Button>
      </div>

      {(showNewConfig || editingConfig) && (
        <CorridorAlertConfigForm
          existingConfig={editingConfig}
          onSaved={handleConfigSaved}
          onCancel={() => { setShowNewConfig(false); setEditingConfig(undefined); }}
        />
      )}

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Total Corridors</CardDescription>
            <CardTitle className="text-3xl">{summaries.length}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <Activity className="h-3 w-3" />
              Monitored corridors
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Critical Corridors</CardDescription>
            <CardTitle className="text-3xl text-red-500">
              {summaries.filter((s) => s.status === "critical").length}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <AlertTriangle className="h-3 w-3" />
              Need immediate attention
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>Active Alert Configs</CardDescription>
            <CardTitle className="text-3xl">{configs.filter((c) => c.is_active).length}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-1 text-xs text-muted-foreground">
              <Settings className="h-3 w-3" />
              Monitoring rules
            </div>
          </CardContent>
        </Card>
      </div>

      {realtimeAlerts.length > 0 && (
        <Card className="border-yellow-500/50 bg-yellow-500/5">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm flex items-center gap-2">
              <Bell className="h-4 w-4 text-yellow-500" />
              Real-time Alerts ({realtimeAlerts.length})
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2 max-h-40 overflow-y-auto">
              {realtimeAlerts.slice(0, 5).map((alert, i) => (
                <div key={i} className="flex items-center gap-2 text-sm">
                  <Badge variant={alert.severity === "critical" ? "destructive" : "warning"}>
                    {alert.severity}
                  </Badge>
                  <span className="truncate">{alert.message}</span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="configs">Alert Configs</TabsTrigger>
          <TabsTrigger value="events">Alert Events</TabsTrigger>
          {selectedCorridor && (
            <TabsTrigger value="detail">Corridor Detail</TabsTrigger>
          )}
        </TabsList>

        <TabsContent value="overview" className="space-y-4">
          {summaries.length === 0 ? (
            <Card>
              <CardContent className="p-6 text-center text-muted-foreground">
                No corridor performance data available yet. Data will appear once corridors are being monitored.
              </CardContent>
            </Card>
          ) : (
            <div className="grid grid-cols-1 gap-4">
              {summaries.map((s) => (
                <Card
                  key={s.corridor_key}
                  className="cursor-pointer hover:border-slate-400 dark:hover:border-slate-600 transition-colors"
                  onClick={() => { setSelectedCorridor(s.corridor_key); setActiveTab("detail"); }}
                >
                  <CardContent className="p-4">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <StatusDot status={s.status} />
                        <div>
                          <div className="font-medium">{s.corridor_key}</div>
                          <div className="text-xs text-muted-foreground">
                            {s.alert_count_24h} alerts in 24h
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-6 text-sm">
                        <div className="text-right">
                          <div className="font-medium">{(s.current_success_rate * 100).toFixed(1)}%</div>
                          <TrendIndicator value={s.success_rate_trend} />
                        </div>
                        <div className="text-right">
                          <div className="font-medium">{s.current_latency_ms.toFixed(0)}ms</div>
                          <TrendIndicator value={s.latency_trend} invert />
                        </div>
                        <div className="text-right">
                          <div className="font-medium">${s.current_liquidity_usd.toLocaleString()}</div>
                          <TrendIndicator value={s.liquidity_trend} />
                        </div>
                        <Badge
                          variant={
                            s.status === "critical"
                              ? "destructive"
                              : s.status === "warning"
                                ? "warning"
                                : "success"
                          }
                        >
                          {s.status}
                        </Badge>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="configs">
          {configs.length === 0 ? (
            <Card>
              <CardContent className="p-6 text-center text-muted-foreground">
                No alert configs yet. Create one to start monitoring corridors.
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-3">
              {configs.map((config) => (
                <Card key={config.id}>
                  <CardContent className="p-4">
                    <div className="flex items-center justify-between">
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="font-medium">{config.name}</span>
                          <Badge variant={config.is_active ? "success" : "outline"}>
                            {config.is_active ? "Active" : "Inactive"}
                          </Badge>
                          {config.corridor_key && (
                            <Badge variant="secondary">{config.corridor_key}</Badge>
                          )}
                        </div>
                        <div className="text-xs text-muted-foreground mt-1">
                          {config.success_rate_threshold && (
                            <span>Success ≥ {(config.success_rate_threshold * 100).toFixed(0)}% </span>
                          )}
                          {config.latency_threshold_ms && (
                            <span>Latency ≤ {config.latency_threshold_ms}ms </span>
                          )}
                          {config.liquidity_threshold_usd && (
                            <span>Liquidity ≥ ${config.liquidity_threshold_usd.toLocaleString()}</span>
                          )}
                        </div>
                        <div className="flex items-center gap-1 mt-1 text-xs text-muted-foreground">
                          Cooldown: {config.cooldown_seconds}s | Channels:
                          {config.notify_in_app && " In-App"}
                          {config.notify_email && " Email"}
                          {config.notify_webhook && " Webhook"}
                          {config.notify_slack && " Slack"}
                          {config.notify_telegram && " Telegram"}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => { setEditingConfig(config); setShowNewConfig(false); }}
                        >
                          Edit
                        </Button>
                        <Button
                          size="sm"
                          variant="destructive"
                          onClick={() => handleDeleteConfig(config.id)}
                        >
                          Delete
                        </Button>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="events">
          <CorridorAlertEventsList />
        </TabsContent>

        {selectedCorridor && (
          <TabsContent value="detail">
            <CorridorDetail corridorKey={selectedCorridor} />
          </TabsContent>
        )}
      </Tabs>
    </div>
  );
}

function CorridorDetail({ corridorKey }: { corridorKey: string }) {
  const [snapshots, setSnapshots] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    corridorAlertsApi
      .getCorridorSnapshots(corridorKey)
      .then(setSnapshots)
      .catch((err) => logger.error("Failed to fetch snapshots:", err))
      .finally(() => setLoading(false));
  }, [corridorKey]);

  if (loading) {
    return <div className="text-muted-foreground py-4">Loading corridor data...</div>;
  }

  return (
    <div className="space-y-4">
      <h3 className="text-lg font-medium">{corridorKey}</h3>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Success Rate</CardTitle>
          </CardHeader>
          <CardContent>
            <CorridorPerformanceChart snapshots={snapshots} metric="success_rate" />
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Latency</CardTitle>
          </CardHeader>
          <CardContent>
            <CorridorPerformanceChart snapshots={snapshots} metric="latency" />
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Liquidity</CardTitle>
          </CardHeader>
          <CardContent>
            <CorridorPerformanceChart snapshots={snapshots} metric="liquidity" />
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">Volume</CardTitle>
          </CardHeader>
          <CardContent>
            <CorridorPerformanceChart snapshots={snapshots} metric="volume" />
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
