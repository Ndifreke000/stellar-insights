import React from 'react';
import { Badge } from '../ui/badge';

interface Asset {
    symbol: string;
    name: string;
    volume24h: number;
    price: number;
    change24h: number;
}

interface TopAssetsTableProps {
    assets: Asset[];
}

export const TopAssetsTable: React.FC<TopAssetsTableProps> = ({ assets }) => {
    return (
        <div className="p-6">
            <div className="flex items-center justify-between mb-6">
                <h3 className="text-sm font-bold uppercase tracking-widest text-muted-foreground">Asset Liquidity // Top Movers</h3>
                <Badge variant="outline" className="text-[10px] font-mono border-border/50">LATEST_SNAPSHOT</Badge>
            </div>

            <div className="relative overflow-x-auto">
                <table className="w-full text-sm text-left">
                    <thead>
                        <tr className="border-b border-border/50">
                            <th className="pb-4 font-bold uppercase tracking-widest text-[10px] text-muted-foreground">Asset Pair</th>
                            <th className="pb-4 font-bold uppercase tracking-widest text-[10px] text-muted-foreground text-right">Price</th>
                            <th className="pb-4 font-bold uppercase tracking-widest text-[10px] text-muted-foreground text-right">Change</th>
                            <th className="pb-4 font-bold uppercase tracking-widest text-[10px] text-muted-foreground text-right">Volume (24h)</th>
                        </tr>
                    </thead>
                    <tbody className="divide-y divide-border/20">
                        {assets.map((asset) => (
                            <tr key={asset.symbol} className="group hover:bg-white/5 transition-colors">
                                <td className="py-4">
                                    <div className="flex items-center gap-3">
                                        <div className="w-8 h-8 rounded-lg bg-accent/10 border border-accent/20 flex items-center justify-center text-accent text-xs font-bold group-hover:glow-accent transition-all">
                                            {asset.symbol.substring(0, 2)}
                                        </div>
                                        <div>
                                            <div className="font-bold tracking-tight">{asset.symbol}</div>
                                            <div className="text-[10px] text-muted-foreground uppercase">{asset.name}</div>
                                        </div>
                                    </div>
                                </td>
                                <td className="py-4 text-right font-mono tabular-nums font-medium">
                                    ${asset.price < 1 ? asset.price.toFixed(4) : asset.price.toLocaleString(undefined, { minimumFractionDigits: 2 })}
                                </td>
                                <td className={`py-4 text-right font-mono tabular-nums font-bold ${asset.change24h >= 0 ? 'text-green-400' : 'text-red-400'}`}>
                                    {asset.change24h > 0 ? '+' : ''}{asset.change24h}%
                                </td>
                                <td className="py-4 text-right font-mono tabular-nums text-muted-foreground">
                                    {new Intl.NumberFormat('en-US', {
                                        style: 'currency',
                                        currency: 'USD',
                                        notation: 'compact'
                                    }).format(asset.volume24h)}
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>
        </div>
    );
};
