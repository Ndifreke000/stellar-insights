"use client";

import { useEffect, useState, useCallback } from "react";
import { AlertTriangle, CheckCircle, Clock, Filter } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { corridorAlertsApi, type CorridorAlertEvent } from "@/lib/alerts-api";
import { logger } from "@/lib/logger";

export function CorridorAlertEventsList() {
  const [events, setEvents] = useState<CorridorAlertEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<"all" | "unacknowledged">("all");

  const fetchEvents = useCallback(async () => {
    try {
      const data = await corridorAlertsApi.getEvents();
      setEvents(data);
    } catch (err) {
      logger.error("Failed to fetch alert events:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchEvents();
  }, [fetchEvents]);

  const handleAcknowledge = async (id: string) => {
    try {
      await corridorAlertsApi.acknowledgeEvent(id);
      setEvents((prev) =>
        prev.map((e) =>
          e.id === id
            ? { ...e, acknowledged: true, acknowledged_at: new Date().toISOString() }
            : e
        )
      );
    } catch (err) {
      logger.error("Failed to acknowledge event:", err);
    }
  };

  const filtered =
    filter === "unacknowledged"
      ? events.filter((e) => !e.acknowledged)
      : events;

  if (loading) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="text-center text-muted-foreground">Loading alert events...</div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Alert Events</CardTitle>
            <CardDescription>
              {events.length} total events, {events.filter((e) => !e.acknowledged).length} unacknowledged
            </CardDescription>
          </div>
          <div className="flex gap-1">
            <Button
              size="sm"
              variant={filter === "all" ? "default" : "outline"}
              onClick={() => setFilter("all")}
            >
              <Filter className="h-3 w-3 mr-1" />
              All
            </Button>
            <Button
              size="sm"
              variant={filter === "unacknowledged" ? "default" : "outline"}
              onClick={() => setFilter("unacknowledged")}
            >
              Unread
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {filtered.length === 0 ? (
          <div className="text-center text-muted-foreground py-8">
            No {filter === "unacknowledged" ? "unread " : ""}alert events
          </div>
        ) : (
          <div className="space-y-3">
            {filtered.map((event) => (
              <div
                key={event.id}
                className={`flex items-start justify-between p-3 rounded-lg border transition-colors ${
                  event.acknowledged
                    ? "bg-slate-50 dark:bg-slate-900/50 opacity-60"
                    : "bg-white dark:bg-slate-950"
                }`}
              >
                <div className="flex items-start gap-3 min-w-0">
                  <div className="mt-0.5">
                    {event.severity === "critical" ? (
                      <AlertTriangle className="h-5 w-5 text-red-500" />
                    ) : (
                      <Clock className="h-5 w-5 text-yellow-500" />
                    )}
                  </div>
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <Badge variant={event.severity === "critical" ? "destructive" : "warning"}>
                        {event.severity}
                      </Badge>
                      <Badge variant="outline">{event.alert_type.replace(/_/g, " ")}</Badge>
                    </div>
                    <p className="text-sm">{event.message}</p>
                    <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                      <span>{event.corridor_key}</span>
                      {event.old_value !== undefined && event.new_value !== undefined && (
                        <span>
                          {event.old_value.toFixed(2)} → {event.new_value.toFixed(2)}
                        </span>
                      )}
                      <span>{new Date(event.triggered_at).toLocaleString()}</span>
                    </div>
                  </div>
                </div>
                {!event.acknowledged && (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => handleAcknowledge(event.id)}
                    className="shrink-0"
                  >
                    <CheckCircle className="h-4 w-4 mr-1" />
                    Ack
                  </Button>
                )}
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
