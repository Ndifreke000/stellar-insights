import { AnchorMetrics } from '@/lib/api';
import { Shield, ShieldAlert, ShieldCheck, Copy, ExternalLink } from 'lucide-react';

interface AnchorHeaderProps {
    anchor: AnchorMetrics;
}

export function AnchorHeader({ anchor }: AnchorHeaderProps) {
    const getStatusColor = (status: string) => {
        switch (status.toLowerCase()) {
            case 'healthy':
                return 'text-emerald-400 bg-emerald-400/10 border-emerald-400/20';
            case 'degraded':
                return 'text-amber-400 bg-amber-400/10 border-amber-400/20';
            case 'unreliable':
                return 'text-rose-400 bg-rose-400/10 border-rose-400/20';
            default:
                return 'text-slate-400 bg-slate-400/10 border-slate-400/20';
        }
    };

    const getStatusIcon = (status: string) => {
        switch (status.toLowerCase()) {
            case 'healthy':
                return <ShieldCheck className="w-5 h-5 text-emerald-400" />;
            case 'degraded':
                return <ShieldAlert className="w-5 h-5 text-amber-400" />;
            case 'unreliable':
                return <ShieldAlert className="w-5 h-5 text-rose-400" />;
            default:
                return <Shield className="w-5 h-5 text-slate-400" />;
        }
    };

    return (
        <div className="bg-slate-900 border border-slate-800 rounded-xl p-6 shadow-sm">
            <div className="flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
                <div>
                    <div className="flex items-center gap-3">
                        <h1 className="text-2xl font-bold text-white tracking-tight">
                            {anchor.name || 'Unknown Anchor'}
                        </h1>
                        <div className={`px-3 py-1 rounded-full text-xs font-medium border flex items-center gap-2 ${getStatusColor(anchor.status)}`}>
                            {getStatusIcon(anchor.status)}
                            {anchor.status}
                        </div>
                    </div>

                    <div className="mt-2 flex items-center gap-2 text-slate-400 text-sm font-mono">
                        <span>{anchor.stellar_account}</span>
                        <button
                            className="p-1 hover:text-white transition-colors"
                            title="Copy Address"
                            onClick={() => navigator.clipboard.writeText(anchor.stellar_account)}
                        >
                            <Copy className="w-3.5 h-3.5" />
                        </button>
                        <a
                            href={`https://stellar.expert/explorer/public/account/${anchor.stellar_account}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="p-1 hover:text-white transition-colors"
                            title="View on Stellar Explorer"
                        >
                            <ExternalLink className="w-3.5 h-3.5" />
                        </a>
                    </div>
                </div>

                <div className="flex items-center gap-6">
                    <div className="text-right">
                        <div className="text-sm text-slate-400 mb-1">Reliability Score</div>
                        <div className="text-3xl font-bold text-white tracking-tight">
                            {anchor.reliability_score.toFixed(1)}
                            <span className="text-lg text-slate-500 font-normal ml-1">/ 100</span>
                        </div>
                    </div>

                    <div className="h-10 w-px bg-slate-800 hidden md:block"></div>

                    <div className="text-right hidden md:block">
                        <div className="text-sm text-slate-400 mb-1">Asset Coverage</div>
                        <div className="text-xl font-semibold text-white">
                            {anchor.asset_coverage} <span className="text-sm text-slate-500 font-normal">assets</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
