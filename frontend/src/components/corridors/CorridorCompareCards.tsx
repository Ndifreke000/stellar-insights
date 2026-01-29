"use client";

import React from "react";
import {
    CheckCircle2,
    AlertCircle,
    Droplets,
    Zap,
    Clock,
    ArrowRight
} from "lucide-react";
import { CorridorMetrics } from "@/lib/api";

interface CompareCardsProps {
    corridors: CorridorMetrics[];
}

export function CorridorCompareCards({ corridors }: CompareCardsProps) {
    const getHealthColor = (score: number) => {
        if (score >= 90) return "text-green-500 bg-green-500/10";
        if (score >= 75) return "text-yellow-500 bg-yellow-500/10";
        return "text-red-500 bg-red-500/10";
    };

    return (
        <div className={`grid grid-cols-1 md:grid-cols-${corridors.length} gap-6 mb-8`}>
            {corridors.map((corridor) => (
                <div
                    key={corridor.id}
                    className="bg-white dark:bg-slate-800 rounded-xl p-6 shadow-lg border border-gray-100 dark:border-slate-700 hover:border-blue-500/50 transition-all"
                >
                    <div className="flex items-center justify-between mb-4">
                        <h3 className="text-xl font-bold text-gray-900 dark:text-white truncate">
                            {corridor.source_asset} <ArrowRight className="inline w-4 h-4 mx-1" /> {corridor.destination_asset}
                        </h3>
                        <div className={`px-2 py-1 rounded text-xs font-bold ${getHealthColor(corridor.health_score)}`}>
                            {corridor.health_score} Score
                        </div>
                    </div>

                    <div className="space-y-4">
                        {/* Success Rate */}
                        <div className="flex justify-between items-end">
                            <div>
                                <p className="text-xs text-gray-500 dark:text-gray-400">Success Rate</p>
                                <div className="flex items-center gap-1">
                                    {corridor.success_rate >= 90 ? (
                                        <CheckCircle2 className="w-4 h-4 text-green-500" />
                                    ) : (
                                        <AlertCircle className="w-4 h-4 text-yellow-500" />
                                    )}
                                    <span className="text-2xl font-bold text-gray-900 dark:text-white">
                                        {corridor.success_rate.toFixed(1)}%
                                    </span>
                                </div>
                            </div>
                        </div>

                        {/* Volume & Liquidity */}
                        <div className="grid grid-cols-2 gap-4">
                            <div>
                                <p className="text-xs text-gray-500 dark:text-gray-400">24h Volume</p>
                                <div className="flex items-center gap-1 text-amber-500">
                                    <Zap className="w-3 h-3" />
                                    <span className="font-bold">
                                        ${(corridor.liquidity_volume_24h_usd / 1000).toFixed(0)}k
                                    </span>
                                </div>
                            </div>
                            <div>
                                <p className="text-xs text-gray-500 dark:text-gray-400">Liquidity</p>
                                <div className="flex items-center gap-1 text-purple-500">
                                    <Droplets className="w-3 h-3" />
                                    <span className="font-bold">
                                        ${(corridor.liquidity_depth_usd / 1000000).toFixed(1)}M
                                    </span>
                                </div>
                            </div>
                        </div>

                        {/* Latency */}
                        <div className="pt-4 border-t border-gray-100 dark:border-slate-700">
                            <div className="flex justify-between items-center text-sm">
                                <span className="text-gray-500 dark:text-gray-400 flex items-center gap-1">
                                    <Clock className="w-4 h-4" /> Avg Latency
                                </span>
                                <span className="font-semibold text-gray-900 dark:text-white">
                                    {corridor.average_latency_ms.toFixed(0)}ms
                                </span>
                            </div>
                            <div className="flex justify-between items-center text-sm mt-1">
                                <span className="text-gray-500 dark:text-gray-400">Avg Slippage</span>
                                <span className="font-semibold text-red-500">
                                    {corridor.average_slippage_bps.toFixed(2)} bps
                                </span>
                            </div>
                        </div>
                    </div>
                </div>
            ))}
        </div>
    );
}

export function BestTimeToTransact({ corridors }: { corridors: CorridorMetrics[] }) {
    // Mocking 'Best time' based on a stable window
    // In a real app, this would use hourly data
    const recommendations = corridors.map(c => {
        let hour = "08:00 UTC";
        let reason = "Lowest historical slippage";

        // Randomize mock a bit for variety
        if (c.success_rate < 90) {
            hour = "14:00 UTC";
            reason = "Peak network stability";
        }

        return { id: c.id, hour, reason };
    });

    return (
        <div className="bg-gradient-to-r from-blue-600 to-indigo-700 rounded-xl p-6 text-white shadow-lg mb-8">
            <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
                <Clock className="w-5 h-5" /> Best Time to Transact
            </h3>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                {recommendations.map(rec => (
                    <div key={rec.id} className="bg-white/10 backdrop-blur-sm rounded-lg p-4 border border-white/20">
                        <p className="text-blue-100 text-xs font-semibold mb-1">{rec.id}</p>
                        <p className="text-2xl font-bold mb-1">{rec.hour}</p>
                        <p className="text-xs text-blue-200">{rec.reason}</p>
                    </div>
                ))}
            </div>
        </div>
    );
}
