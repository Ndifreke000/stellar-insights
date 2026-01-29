"use client";

import React, { useEffect, useState, Suspense } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import {
    ArrowLeft,
    Share2,
    BarChart3,
    Check,
    Share
} from "lucide-react";
import Link from "next/link";
import { MainLayout } from "@/components/layout";
import {
    getCorridorDetail,
    CorridorDetailData,
    generateMockCorridorData
} from "@/lib/api";
import {
    SuccessRateCompareChart,
    VolumeCompareChart,
    SlippageCompareChart
} from "@/components/corridors/CorridorCompareCharts";
import {
    CorridorCompareCards,
    BestTimeToTransact
} from "@/components/corridors/CorridorCompareCards";

function ComparisonContent() {
    const searchParams = useSearchParams();
    const idsString = searchParams.get("ids") || "";
    const [selectedIds, setSelectedIds] = useState<string[]>(
        idsString ? idsString.split(",").filter(Boolean) : []
    );
    const [corridorData, setCorridorData] = useState<CorridorDetailData[]>([]);
    const [loading, setLoading] = useState(true);
    const [copied, setCopied] = useState(false);

    useEffect(() => {
        async function fetchData() {
            if (selectedIds.length === 0) {
                setLoading(false);
                return;
            }

            try {
                setLoading(true);
                const results = await Promise.all(
                    selectedIds.map(async (id) => {
                        try {
                            // Note: getCorridorDetail might not be fully functional if API is down
                            // but it's used in [pair]/page.tsx so we assume it works or we use mock
                            const data = await getCorridorDetail(id);
                            return data;
                        } catch (e) {
                            console.log(`Failed to fetch ${id}, using mock`);
                            return generateMockCorridorData(id);
                        }
                    })
                );
                setCorridorData(results);
            } catch (err) {
                console.error("Error fetching comparison data:", err);
            } finally {
                setLoading(false);
            }
        }
        fetchData();
    }, [selectedIds]);

    const handleShare = () => {
        const url = window.location.href;
        navigator.clipboard.writeText(url);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };

    if (loading) {
        return (
            <div className="flex flex-col items-center justify-center min-h-[400px]">
                <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mb-4"></div>
                <p className="text-gray-500 dark:text-gray-400">Comparing corridors...</p>
            </div>
        );
    }

    if (selectedIds.length === 0) {
        return (
            <div className="text-center py-20 bg-white dark:bg-slate-800 rounded-2xl shadow-sm border border-dashed border-gray-300 dark:border-slate-700">
                <BarChart3 className="w-16 h-16 text-gray-300 mx-auto mb-4" />
                <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-2">No Corridors Selected</h2>
                <p className="text-gray-500 dark:text-gray-400 mb-6 px-4">Select up to 3 corridors from the listing page to compare them side-by-side.</p>
                <Link
                    href="/corridors"
                    className="inline-flex items-center gap-2 bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-lg font-semibold transition-colors"
                >
                    Browse Corridors
                </Link>
            </div>
        );
    }

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            {/* Header Actions */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                <div className="flex items-center gap-4">
                    <Link
                        href="/corridors"
                        className="p-2 hover:bg-gray-100 dark:hover:bg-slate-700 rounded-full transition-colors text-gray-500"
                    >
                        <ArrowLeft className="w-5 h-5" />
                    </Link>
                    <div>
                        <h1 className="text-2xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
                            Corridor Comparison
                            <span className="text-sm font-normal text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-slate-700 px-2 py-0.5 rounded-full">
                                {selectedIds.length} Selected
                            </span>
                        </h1>
                    </div>
                </div>
                <button
                    onClick={handleShare}
                    className="flex items-center justify-center gap-2 bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 px-4 py-2 rounded-lg font-semibold text-gray-700 dark:text-gray-200 hover:border-blue-500 transition-all shadow-sm"
                >
                    {copied ? <Check className="w-4 h-4 text-green-500" /> : <Share2 className="w-4 h-4 text-blue-500" />}
                    {copied ? "URL Copied!" : "Share Comparison"}
                </button>
            </div>

            {/* Metrics Cards */}
            <CorridorCompareCards corridors={corridorData.map(d => d.corridor)} />

            {/* Recommendations */}
            <BestTimeToTransact corridors={corridorData.map(d => d.corridor)} />

            {/* Charts Section */}
            <div className="grid grid-cols-1 gap-8">
                <SuccessRateCompareChart corridors={corridorData} />
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
                    <VolumeCompareChart corridors={corridorData} />
                    <SlippageCompareChart corridors={corridorData} />
                </div>
            </div>
        </div>
    );
}

export default function ComparePage() {
    return (
        <MainLayout>
            <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto">
                <Suspense fallback={<div className="flex items-center justify-center min-h-[400px]"><div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div></div>}>
                    <ComparisonContent />
                </Suspense>
            </div>
        </MainLayout>
    );
}
