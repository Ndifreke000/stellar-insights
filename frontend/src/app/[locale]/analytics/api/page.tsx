"use client";

import React, { useEffect, useState } from "react";
import { RefreshCw, Activity, ChevronLeft } from "lucide-react";
import { Link } from "@/i18n/navigation";
import { fetchApiUsageOverview, ApiUsageOverview } from "@/lib/analytics-api";
import { ApiUsageDashboard } from "@/components/analytics/ApiUsageDashboard";

export default function ApiAnalyticsPage() {
    const [data, setData] = useState<ApiUsageOverview | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

    const loadData = async () => {
        try {
            setLoading(true);
            setError(null);
            const overview = await fetchApiUsageOverview();
            setData(overview);
            setLastUpdated(new Date());
        } catch (err) {
            setError(err instanceof Error ? err.message : "Failed to load API analytics");
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadData();
        const interval = setInterval(loadData, 30000); // Auto-refresh every 30 seconds
        return () => clearInterval(interval);
    }, []);

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
            {/* Page Header */}
            <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
                <div>
                    <div className="flex items-center gap-2 mb-2">
                        <Link
                            href="/analytics"
                            className="p-1.5 glass rounded-lg hover:bg-white/10 transition-colors text-muted-foreground hover:text-white"
                        >
                            <ChevronLeft className="w-4 h-4" />
                        </Link>
                        <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em]">Telemetry // System</div>
                    </div>
                    <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
                        <Activity className="w-8 h-8 text-blue-500" />
                        API Usage Metrics
                    </h2>
                </div>
                <div className="flex items-center gap-3">
                    <div className="px-4 py-2 glass rounded-lg text-[10px] font-mono uppercase tracking-widest text-muted-foreground">
                        Last Scan: {lastUpdated?.toLocaleTimeString()}
                    </div>
                    <button
                        onClick={loadData}
                        disabled={loading}
                        className="px-4 py-2 bg-accent text-white rounded-lg text-[10px] font-bold uppercase tracking-widest hover:scale-105 transition-transform flex items-center gap-2 disabled:opacity-50"
                    >
                        <RefreshCw className={`w-3 h-3 ${loading ? "animate-spin" : ""}`} />
                        Refresh
                    </button>
                </div>
            </div>

            {error ? (
                <div className="glass border-red-500/30 p-8 rounded-2xl text-center space-y-4">
                    <div className="text-red-500 font-mono uppercase tracking-widest text-sm italic">
                        Telemetry Failure // Link Unavailable
                    </div>
                    <p className="text-muted-foreground max-w-md mx-auto">{error}</p>
                    <button
                        onClick={loadData}
                        className="px-6 py-2 border border-white/10 rounded-lg text-[10px] uppercase tracking-widest hover:bg-white/5 transition-all"
                    >
                        Retry Scan
                    </button>
                </div>
            ) : loading && !data ? (
                <div className="flex h-[40vh] items-center justify-center">
                    <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest italic tracking-widest">
                        Intercepting Data Packets... // SI-9
                    </div>
                </div>
            ) : data ? (
                <ApiUsageDashboard data={data} />
            ) : null}
        </div>
    );
}
