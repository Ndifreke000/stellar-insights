"use client";

import React from "react";
import {
    BarChart,
    Bar,
    XAxis,
    YAxis,
    CartesianGrid,
    Tooltip,
    ResponsiveContainer,
    Cell,
    PieChart,
    Pie,
} from "recharts";
import { ApiUsageOverview } from "@/lib/analytics-api";
import { Activity, Clock, AlertTriangle, ShieldCheck } from "lucide-react";

interface ApiUsageDashboardProps {
    data: ApiUsageOverview;
}

const COLORS = ["#0088FE", "#00C49F", "#FFBB28", "#FF8042", "#8884d8"];

export const ApiUsageDashboard: React.FC<ApiUsageDashboardProps> = ({ data }) => {
    const statusColors: Record<number, string> = {
        200: "#10b981",
        201: "#10b981",
        404: "#f59e0b",
        500: "#ef4444",
        403: "#ef4444",
        401: "#ef4444",
    };

    return (
        <div className="space-y-6">
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                <AnalyticsCard
                    icon={<Activity className="w-5 h-5 text-blue-500" />}
                    label="Total Requests"
                    value={data.total_requests.toLocaleString()}
                    color="blue"
                />
                <AnalyticsCard
                    icon={<Clock className="w-5 h-5 text-emerald-500" />}
                    label="Avg Latency"
                    value={`${data.avg_response_time_ms.toFixed(1)}ms`}
                    color="emerald"
                />
                <AnalyticsCard
                    icon={<AlertTriangle className="w-5 h-5 text-amber-500" />}
                    label="Error Rate"
                    value={`${data.error_rate.toFixed(1)}%`}
                    color="amber"
                />
                <AnalyticsCard
                    icon={<ShieldCheck className="w-5 h-5 text-purple-500" />}
                    label="System Health"
                    value={data.error_rate < 5 ? "HEALTHY" : "DEGRADED"}
                    color="purple"
                />
            </div>

            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                {/* Top Endpoints */}
                <div className="glass-card rounded-2xl p-6">
                    <h3 className="text-lg font-bold uppercase tracking-tighter italic mb-4">
                        Populous Endpoints // Statistics
                    </h3>
                    <div className="h-[300px]">
                        <ResponsiveContainer width="100%" height="100%">
                            <BarChart data={data.top_endpoints} layout="vertical">
                                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                                <XAxis type="number" stroke="#888" fontSize={10} />
                                <YAxis
                                    dataKey="endpoint"
                                    type="category"
                                    stroke="#888"
                                    fontSize={10}
                                    width={100}
                                />
                                <Tooltip
                                    contentStyle={{ backgroundColor: "#111", border: "1px solid #333" }}
                                    itemStyle={{ fontSize: "12px" }}
                                />
                                <Bar dataKey="count" fill="#3b82f6" radius={[0, 4, 4, 0]} />
                            </BarChart>
                        </ResponsiveContainer>
                    </div>
                </div>

                {/* Status Code Distribution */}
                <div className="glass-card rounded-2xl p-6">
                    <h3 className="text-lg font-bold uppercase tracking-tighter italic mb-4">
                        Response Status // Distribution
                    </h3>
                    <div className="h-[300px]">
                        <ResponsiveContainer width="100%" height="100%">
                            <PieChart>
                                <Pie
                                    data={data.status_distribution}
                                    dataKey="count"
                                    nameKey="status_code"
                                    cx="50%"
                                    cy="50%"
                                    outerRadius={100}
                                    fill="#8884d8"
                                    label={(entry) => `${entry.status_code}: ${entry.count}`}
                                >
                                    {data.status_distribution.map((entry, index) => (
                                        <Cell
                                            key={`cell-${index}`}
                                            fill={statusColors[entry.status_code] || COLORS[index % COLORS.length]}
                                        />
                                    ))}
                                </Pie>
                                <Tooltip
                                    contentStyle={{ backgroundColor: "#111", border: "1px solid #333" }}
                                />
                            </PieChart>
                        </ResponsiveContainer>
                    </div>
                </div>
            </div>

            {/* Latency by Endpoint */}
            <div className="glass-card rounded-2xl p-6">
                <h3 className="text-lg font-bold uppercase tracking-tighter italic mb-4">
                    Latency Diagnostics // Per Endpoint
                </h3>
                <div className="overflow-x-auto">
                    <table className="w-full text-left text-sm font-mono uppercase tracking-widest">
                        <thead className="border-b border-border/50 text-[10px] text-muted-foreground">
                            <tr>
                                <th className="py-3 px-2">Endpoint</th>
                                <th className="py-3 px-2 text-center">Method</th>
                                <th className="py-3 px-2 text-right">Calls</th>
                                <th className="py-3 px-2 text-right">Avg Latency</th>
                            </tr>
                        </thead>
                        <tbody>
                            {data.top_endpoints.map((ep, idx) => (
                                <tr key={idx} className="border-b border-border/20 hover:bg-white/5 transition-colors">
                                    <td className="py-3 px-2 truncate max-w-[200px]">{ep.endpoint}</td>
                                    <td className="py-3 px-2 text-center">
                                        <span className={`px-2 py-0.5 rounded text-[10px] bg-blue-500/10 text-blue-400 border border-blue-500/20`}>
                                            {ep.method}
                                        </span>
                                    </td>
                                    <td className="py-3 px-2 text-right font-bold text-accent">{ep.count}</td>
                                    <td className="py-3 px-2 text-right">
                                        <span className={ep.avg_response_time_ms > 200 ? "text-amber-500" : "text-emerald-500"}>
                                            {ep.avg_response_time_ms.toFixed(1)}ms
                                        </span>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    );
};

interface AnalyticsCardProps {
    icon: React.ReactNode;
    label: string;
    value: string;
    color: string;
}

const AnalyticsCard: React.FC<AnalyticsCardProps> = ({ icon, label, value, color }) => {
    const bgColors: Record<string, string> = {
        blue: "bg-blue-500/10",
        emerald: "bg-emerald-500/10",
        amber: "bg-amber-500/10",
        purple: "bg-purple-500/10",
    };

    return (
        <div className="glass rounded-2xl p-6 group hover:scale-[1.02] transition-transform">
            <div className={`w-10 h-10 rounded-xl ${bgColors[color]} flex items-center justify-center mb-4`}>
                {icon}
            </div>
            <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-[0.2em] mb-1">
                {label}
            </div>
            <div className="text-2xl font-black italic tracking-tighter uppercase whitespace-break-spaces">
                {value}
            </div>
        </div>
    );
};
