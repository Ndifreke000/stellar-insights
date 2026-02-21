"use client";

import React, { useEffect, useState, Suspense } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import {
    ArrowLeft,
    Share2,
    BarChart3,
    Check,
    Download,
    Plus,
    X
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
import { CorridorComparisonTable } from "@/components/CorridorComparisonTable";

function ComparisonContent() {
    const searchParams = useSearchParams();
    const router = useRouter();
    const idsString = searchParams.get("ids") || "";
    const [selectedIds, setSelectedIds] = useState<string[]>(
        idsString ? idsString.split(",").filter(Boolean) : []
    );
    const [corridorData, setCorridorData] = useState<CorridorDetailData[]>([]);
    const [loading, setLoading] = useState(true);
    const [copied, setCopied] = useState(false);
    const [showAddModal, setShowAddModal] = useState(false);
    const [newCorridorId, setNewCorridorId] = useState("");

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

    const handleExportCSV = () => {
        if (corridorData.length === 0) return;

        // Create CSV content
        const headers = [
            'Corridor',
            'Success Rate (%)',
            'Health Score',
            'Avg Latency (ms)',
            'Liquidity Depth (USD)',
            '24h Volume (USD)',
            'Avg Slippage (bps)',
            'Total Attempts',
            'Successful',
            'Failed'
        ];

        const rows = corridorData.map(d => [
            `${d.corridor.source_asset}-${d.corridor.destination_asset}`,
            d.corridor.success_rate.toFixed(2),
            d.corridor.health_score.toFixed(1),
            d.corridor.average_latency_ms.toFixed(0),
            d.corridor.liquidity_depth_usd.toFixed(2),
            d.corridor.liquidity_volume_24h_usd.toFixed(2),
            d.corridor.average_slippage_bps.toFixed(2),
            d.corridor.total_attempts,
            d.corridor.successful_payments,
            d.corridor.failed_payments
        ]);

        const csvContent = [
            headers.join(','),
            ...rows.map(row => row.join(','))
        ].join('\n');

        // Download CSV
        const blob = new Blob([csvContent], { type: 'text/csv' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `corridor-comparison-${new Date().toISOString().split('T')[0]}.csv`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    };

    const handleAddCorridor = () => {
        if (!newCorridorId.trim()) return;
        if (selectedIds.includes(newCorridorId.trim())) {
            alert('This corridor is already in the comparison');
            return;
        }
        if (selectedIds.length >= 4) {
            alert('Maximum 4 corridors can be compared at once');
            return;
        }

        const newIds = [...selectedIds, newCorridorId.trim()];
        setSelectedIds(newIds);
        router.push(`/corridors/compare?ids=${newIds.join(',')}`);
        setNewCorridorId('');
        setShowAddModal(false);
    };

    const handleRemoveCorridor = (idToRemove: string) => {
        const newIds = selectedIds.filter(id => id !== idToRemove);
        setSelectedIds(newIds);
        if (newIds.length > 0) {
            router.push(`/corridors/compare?ids=${newIds.join(',')}`);
        } else {
            router.push('/corridors/compare');
        }
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
                        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                            Compare up to 4 corridors side-by-side
                        </p>
                    </div>
                </div>
                <div className="flex items-center gap-2">
                    {selectedIds.length < 4 && (
                        <button
                            onClick={() => setShowAddModal(true)}
                            className="flex items-center gap-2 bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 px-4 py-2 rounded-lg font-semibold text-gray-700 dark:text-gray-200 hover:border-blue-500 transition-all shadow-sm"
                        >
                            <Plus className="w-4 h-4" />
                            Add Corridor
                        </button>
                    )}
                    <button
                        onClick={handleExportCSV}
                        className="flex items-center gap-2 bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 px-4 py-2 rounded-lg font-semibold text-gray-700 dark:text-gray-200 hover:border-green-500 transition-all shadow-sm"
                    >
                        <Download className="w-4 h-4 text-green-500" />
                        Export CSV
                    </button>
                    <button
                        onClick={handleShare}
                        className="flex items-center gap-2 bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 px-4 py-2 rounded-lg font-semibold text-gray-700 dark:text-gray-200 hover:border-blue-500 transition-all shadow-sm"
                    >
                        {copied ? <Check className="w-4 h-4 text-green-500" /> : <Share2 className="w-4 h-4 text-blue-500" />}
                        {copied ? "Copied!" : "Share"}
                    </button>
                </div>
            </div>

            {/* Selected Corridors Pills */}
            {selectedIds.length > 0 && (
                <div className="flex flex-wrap gap-2">
                    {selectedIds.map((id) => (
                        <div
                            key={id}
                            className="flex items-center gap-2 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700 px-3 py-1 rounded-full text-sm"
                        >
                            <span className="font-medium text-blue-700 dark:text-blue-300">{id}</span>
                            <button
                                onClick={() => handleRemoveCorridor(id)}
                                className="text-blue-500 hover:text-blue-700 dark:hover:text-blue-400"
                            >
                                <X className="w-4 h-4" />
                            </button>
                        </div>
                    ))}
                </div>
            )}

            {/* Metrics Cards */}
            <CorridorCompareCards corridors={corridorData.map(d => d.corridor)} />

            {/* Comparison Table */}
            <CorridorComparisonTable 
                corridors={corridorData.map(d => d.corridor)}
                onExport={handleExportCSV}
            />

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

            {/* Add Corridor Modal */}
            {showAddModal && (
                <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div className="bg-white dark:bg-slate-800 rounded-lg p-6 max-w-md w-full mx-4">
                        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                            Add Corridor to Comparison
                        </h3>
                        <input
                            type="text"
                            value={newCorridorId}
                            onChange={(e) => setNewCorridorId(e.target.value)}
                            placeholder="Enter corridor ID (e.g., USDC-XLM)"
                            className="w-full px-4 py-2 border border-gray-300 dark:border-slate-600 rounded-lg bg-white dark:bg-slate-700 text-gray-900 dark:text-white mb-4"
                            onKeyPress={(e) => e.key === 'Enter' && handleAddCorridor()}
                        />
                        <div className="flex gap-2">
                            <button
                                onClick={() => setShowAddModal(false)}
                                className="flex-1 px-4 py-2 border border-gray-300 dark:border-slate-600 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-slate-700"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={handleAddCorridor}
                                className="flex-1 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                            >
                                Add
                            </button>
                        </div>
                    </div>
                </div>
            )}
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
