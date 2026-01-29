'use client';

import React, { useEffect, useState, use, Suspense } from 'react';
import { getAnchorDetail, AnchorDetailData } from '@/lib/api';
import { AnchorHeader } from '@/components/anchors/AnchorHeader';
import { IssuedAssetsTable } from '@/components/anchors/IssuedAssetsTable';
import { ReliabilityTrend } from '@/components/charts/ReliabilityTrend';
import { AlertCircle, Clock, ArrowLeft, XCircle } from 'lucide-react';
import Link from 'next/link';

function AnchorDetailPageContent({ params }: { params: Promise<{ address: string }> }) {
    const unwrappedParams = use(params);
    const { address } = unwrappedParams;
    const [data, setData] = useState<AnchorDetailData | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        async function fetchData() {
            if (!address) return;

            // Validate Stellar address format (G... and 56 chars)
            if (!/^[G][A-Z0-9]{55}$/.test(address)) {
                setError('Invalid anchor address format. Expected a 56-character Stellar public key starting with G.');
                setLoading(false);
                return;
            }

            try {
                setLoading(true);
                const result = await getAnchorDetail(address);
                setData(result);
                setError(null);
            } catch (err) {
                console.error('Failed to fetch anchor details:', err);
                setError('Failed to load anchor data. Please try again later.');
            } finally {
                setLoading(false);
            }
        }

        fetchData();
    }, [address]);

    if (loading) {
        return (
            <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
                <div className="animate-pulse">
                    <div className="h-4 w-32 bg-slate-800 rounded mb-6"></div>
                    <div className="h-32 bg-slate-800 rounded-xl mb-8"></div>
                    <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-8">
                        <div className="h-80 bg-slate-800 rounded-xl"></div>
                        <div className="h-80 bg-slate-800 rounded-xl"></div>
                    </div>
                </div>
            </div>
        );
    }

    if (error || !data) {
        return (
            <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16 flex flex-col items-center justify-center text-center">
                <div className="w-16 h-16 bg-rose-500/10 rounded-full flex items-center justify-center mb-4">
                    <AlertCircle className="w-8 h-8 text-rose-500" />
                </div>
                <h2 className="text-xl font-bold text-white mb-2">Error Loading Anchor</h2>
                <p className="text-slate-400 mb-6 max-w-md">{error || 'Anchor not found.'}</p>
                <Link
                    href="/anchors"
                    className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-white rounded-lg transition-colors font-medium text-sm"
                >
                    Return to Anchors List
                </Link>
            </div>
        );
    }

    return (
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 space-y-8">
            {/* Breadcrumb / Back */}
            <div className="flex items-center gap-2 text-sm text-slate-400">
                <Link href="/anchors" className="hover:text-white transition-colors flex items-center gap-1">
                    <ArrowLeft className="w-4 h-4" />
                    Anchors
                </Link>
                <span className="text-slate-600">/</span>
                <span className="text-slate-200 truncate max-w-[200px]">{data.anchor.name || address}</span>
            </div>

            <AnchorHeader anchor={data.anchor} />

            {/* Main Grid */}
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
                {/* Left Column: Reliability Trend (Span 2) */}
                <div className="lg:col-span-2">
                    <ReliabilityTrend data={data.reliability_history} />
                </div>

                {/* Right Column: Failure Diagnostics (Span 1) */}
                <div className="lg:col-span-1 bg-slate-900 border border-slate-800 rounded-xl p-6 shadow-sm">
                    <h3 className="font-semibold text-white mb-4 flex items-center gap-2">
                        <XCircle className="w-4 h-4 text-rose-400" />
                        Failure Diagnostics
                    </h3>

                    <div className="space-y-6">
                        {/* Top Failure Reasons */}
                        <div>
                            <h4 className="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-3">
                                Top Failure Reasons
                            </h4>
                            <div className="space-y-3">
                                {data.top_failure_reasons?.map((item, i) => (
                                    <div key={i} className="flex justify-between items-center text-sm">
                                        <span className="text-slate-300 truncate pr-4" title={item.reason}>
                                            {item.reason}
                                        </span>
                                        <span className="font-mono text-rose-400 bg-rose-400/10 px-2 py-0.5 rounded text-xs">
                                            {item.count}
                                        </span>
                                    </div>
                                )) || <div className="text-sm text-slate-500 italic">No failure data available</div>}
                            </div>
                        </div>

                        <div className="h-px bg-slate-800"></div>

                        {/* Recent Failed Corridors */}
                        <div>
                            <h4 className="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-3">
                                Recent Failed Corridors
                            </h4>
                            <div className="space-y-3">
                                {data.recent_failed_corridors?.map((item, i) => (
                                    <div key={i} className="flex justify-between items-start text-sm">
                                        <span className="text-indigo-400 font-medium">{item.corridor_id}</span>
                                        <span className="text-slate-500 text-xs flex items-center gap-1">
                                            <Clock className="w-3 h-3" />
                                            {new Date(item.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                                        </span>
                                    </div>
                                )) || <div className="text-sm text-slate-500 italic">No recent failures</div>}
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Issued Assets Table (Full Width) */}
            <div>
                <IssuedAssetsTable assets={data.issued_assets} />
            </div>
        </div>
    );
}

export default function AnchorDetailPage(props: { params: Promise<{ address: string }> }) {
    return (
        <Suspense fallback={
            <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 flex items-center justify-center min-h-[400px]">
                <div className="w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
            </div>
        }>
            <AnchorDetailPageContent {...props} />
        </Suspense>
    );
}
