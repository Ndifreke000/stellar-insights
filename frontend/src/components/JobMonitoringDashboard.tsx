"use client";

import React, { useState, useEffect } from "react";
import { AlertTriangle, CheckCircle, Clock, Activity, AlertCircle, TrendingUp, TrendingDown } from "lucide-react";

interface JobStatus {
  name: string;
  is_active: boolean;
  total_executions: number;
  total_failures: number;
  consecutive_failures: number;
  success_rate: number;
  last_success_timestamp: number | null;
  last_failure_timestamp: number | null;
  last_execution: {
    status: string;
    started_at: number;
    duration_ms: number | null;
    completed_at: number | null;
    error: string | null;
  } | null;
  health_status: "healthy" | "warning" | "critical" | "unknown";
}

interface JobSummary {
  total_jobs: number;
  active_jobs: number;
  healthy_jobs: number;
  warning_jobs: number;
  critical_jobs: number;
  total_executions: number;
  total_failures: number;
  overall_success_rate: number;
}

interface JobStatusResponse {
  jobs: Record<string, JobStatus>;
  summary: JobSummary;
  timestamp: number;
}

export function JobMonitoringDashboard() {
  const [jobData, setJobData] = useState<JobStatusResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);

  const fetchJobStatus = async () => {
    try {
      const response = await fetch("/api/v1/jobs/status");
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data = await response.json();
      setJobData(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch job status");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchJobStatus();
  }, []);

  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(fetchJobStatus, 30000); // Refresh every 30 seconds
    return () => clearInterval(interval);
  }, [autoRefresh]);

  const formatTimestamp = (timestamp: number | null) => {
    if (!timestamp) return "Never";
    return new Date(timestamp * 1000).toLocaleString();
  };

  const formatDuration = (durationMs: number | null) => {
    if (!durationMs) return "N/A";
    if (durationMs < 1000) return `${durationMs}ms`;
    if (durationMs < 60000) return `${(durationMs / 1000).toFixed(1)}s`;
    return `${(durationMs / 60000).toFixed(1)}m`;
  };

  const getHealthIcon = (status: string) => {
    switch (status) {
      case "healthy":
        return <CheckCircle className="w-4 h-4 text-emerald-400" />;
      case "warning":
        return <AlertTriangle className="w-4 h-4 text-amber-400" />;
      case "critical":
        return <AlertCircle className="w-4 h-4 text-red-400" />;
      default:
        return <Clock className="w-4 h-4 text-gray-400" />;
    }
  };

  const getHealthColor = (status: string) => {
    switch (status) {
      case "healthy":
        return "text-emerald-400";
      case "warning":
        return "text-amber-400";
      case "critical":
        return "text-red-400";
      default:
        return "text-gray-400";
    }
  };

  if (loading && !jobData) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Activity className="w-5 h-5 animate-pulse" />
          Loading job status...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-center">
          <AlertCircle className="w-12 h-12 text-red-400 mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-foreground mb-2">
            Error Loading Job Status
          </h3>
          <p className="text-muted-foreground">{error}</p>
          <button
            onClick={fetchJobStatus}
            className="mt-4 px-4 py-2 bg-accent text-accent-foreground rounded-lg hover:opacity-90"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  if (!jobData) return null;

  const { jobs, summary } = jobData;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-foreground">Job Monitoring</h2>
          <p className="text-muted-foreground">
            Real-time job execution status and health monitoring
          </p>
        </div>
        <div className="flex items-center gap-4">
          <button
            onClick={() => setAutoRefresh(!autoRefresh)}
            className={`px-4 py-2 rounded-lg border ${
              autoRefresh
                ? "bg-accent border-accent text-accent-foreground"
                : "border-border text-foreground hover:bg-white/5"
            }`}
          >
            Auto Refresh: {autoRefresh ? "On" : "Off"}
          </button>
          <button
            onClick={fetchJobStatus}
            className="px-4 py-2 bg-accent text-accent-foreground rounded-lg hover:opacity-90"
          >
            Refresh Now
          </button>
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="glass-card rounded-xl p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-muted-foreground">Total Jobs</span>
            <Activity className="w-4 h-4 text-muted-foreground" />
          </div>
          <div className="text-2xl font-bold text-foreground">{summary.total_jobs}</div>
        </div>

        <div className="glass-card rounded-xl p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-muted-foreground">Healthy</span>
            <CheckCircle className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="text-2xl font-bold text-emerald-400">{summary.healthy_jobs}</div>
        </div>

        <div className="glass-card rounded-xl p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-muted-foreground">Warnings</span>
            <AlertTriangle className="w-4 h-4 text-amber-400" />
          </div>
          <div className="text-2xl font-bold text-amber-400">{summary.warning_jobs}</div>
        </div>

        <div className="glass-card rounded-xl p-6">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-muted-foreground">Critical</span>
            <AlertCircle className="w-4 h-4 text-red-400" />
          </div>
          <div className="text-2xl font-bold text-red-400">{summary.critical_jobs}</div>
        </div>
      </div>

      {/* Overall Stats */}
      <div className="glass-card rounded-xl p-6">
        <h3 className="text-lg font-semibold text-foreground mb-4">Overall Statistics</h3>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div>
            <div className="text-sm text-muted-foreground mb-1">Success Rate</div>
            <div className="flex items-center gap-2">
              <span className="text-2xl font-bold text-foreground">
                {summary.overall_success_rate.toFixed(1)}%
              </span>
              {summary.overall_success_rate >= 80 ? (
                <TrendingUp className="w-5 h-5 text-emerald-400" />
              ) : (
                <TrendingDown className="w-5 h-5 text-red-400" />
              )}
            </div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground mb-1">Total Executions</div>
            <div className="text-2xl font-bold text-foreground">{summary.total_executions}</div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground mb-1">Total Failures</div>
            <div className="text-2xl font-bold text-red-400">{summary.total_failures}</div>
          </div>
        </div>
      </div>

      {/* Job Details */}
      <div className="glass-card rounded-xl p-6">
        <h3 className="text-lg font-semibold text-foreground mb-4">Job Details</h3>
        <div className="space-y-4">
          {Object.values(jobs)
            .sort((a, b) => {
              // Sort by health status (critical first, then warning, then healthy)
              const statusOrder = { critical: 0, warning: 1, healthy: 2, unknown: 3 };
              return statusOrder[a.health_status] - statusOrder[b.health_status];
            })
            .map((job) => (
              <div
                key={job.name}
                className="border border-border/60 rounded-lg p-4 hover:bg-white/5 transition-colors"
              >
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    {getHealthIcon(job.health_status)}
                    <div>
                      <h4 className="font-medium text-foreground">{job.name}</h4>
                      <div className="flex items-center gap-2 text-sm text-muted-foreground">
                        {job.is_active && (
                          <span className="px-2 py-1 bg-emerald-500/20 text-emerald-400 rounded text-xs">
                            Active
                          </span>
                        )}
                        <span className={`px-2 py-1 rounded text-xs ${getHealthColor(job.health_status)}`}>
                          {job.health_status}
                        </span>
                      </div>
                    </div>
                  </div>
                  <div className="text-right text-sm text-muted-foreground">
                    {job.success_rate.toFixed(1)}% success rate
                  </div>
                </div>

                <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                  <div>
                    <div className="text-muted-foreground">Executions</div>
                    <div className="font-medium">{job.total_executions}</div>
                  </div>
                  <div>
                    <div className="text-muted-foreground">Failures</div>
                    <div className="font-medium text-red-400">{job.total_failures}</div>
                  </div>
                  <div>
                    <div className="text-muted-foreground">Consecutive Failures</div>
                    <div className="font-medium text-amber-400">{job.consecutive_failures}</div>
                  </div>
                  <div>
                    <div className="text-muted-foreground">Last Success</div>
                    <div className="font-medium">{formatTimestamp(job.last_success_timestamp)}</div>
                  </div>
                </div>

                {job.last_execution && (
                  <div className="mt-3 pt-3 border-t border-border/60">
                    <div className="text-sm text-muted-foreground mb-1">Last Execution</div>
                    <div className="flex items-center justify-between">
                      <span className="capitalize">{job.last_execution.status}</span>
                      <span>{formatDuration(job.last_execution.duration_ms)}</span>
                    </div>
                    {job.last_execution.error && (
                      <div className="mt-1 text-xs text-red-400">
                        Error: {job.last_execution.error}
                      </div>
                    )}
                  </div>
                )}
              </div>
            ))}
        </div>
      </div>

      {/* Footer */}
      <div className="text-center text-sm text-muted-foreground">
        Last updated: {formatTimestamp(jobData.timestamp)}
      </div>
    </div>
  );
}
