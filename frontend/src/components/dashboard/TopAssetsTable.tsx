import React from 'react';

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
        <div className="bg-card text-card-foreground rounded-xl border shadow-sm">
            <div className="flex flex-col space-y-1.5 p-6">
                <h3 className="font-semibold leading-none tracking-tight">Top Performing Assets</h3>
                <p className="text-sm text-muted-foreground">Assets sorted by 24h volume.</p>
            </div>
            <div className="p-6 pt-0 relative w-full overflow-auto">
                <table className="w-full caption-bottom text-sm">
                    <thead className="[&_tr]:border-b">
                        <tr className="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted">
                            <th className="h-12 px-4 text-left align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0">Asset</th>
                            <th className="h-12 px-4 text-right align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0">Price</th>
                            <th className="h-12 px-4 text-right align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0">24h Change</th>
                            <th className="h-12 px-4 text-right align-middle font-medium text-muted-foreground [&:has([role=checkbox])]:pr-0">24h Vol</th>
                        </tr>
                    </thead>
                    <tbody className="[&_tr:last-child]:border-0">
                        {assets.map((asset) => (
                            <tr key={asset.symbol} className="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted">
                                <td className="p-4 align-middle [&:has([role=checkbox])]:pr-0 font-medium">
                                    <div className="flex items-center">
                                        <div className="bg-primary/10 text-primary rounded-full w-8 h-8 flex items-center justify-center mr-3 text-xs font-bold">
                                            {asset.symbol[0]}
                                        </div>
                                        <div>
                                            <div className="font-bold">{asset.symbol}</div>
                                            <div className="text-xs text-muted-foreground">{asset.name}</div>
                                        </div>
                                    </div>
                                </td>
                                <td className="p-4 align-middle [&:has([role=checkbox])]:pr-0 text-right">
                                    ${asset.price < 1 ? asset.price.toFixed(4) : asset.price.toFixed(2)}
                                </td>
                                <td className={`p-4 align-middle [&:has([role=checkbox])]:pr-0 text-right ${asset.change24h >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                                    {asset.change24h > 0 ? '+' : ''}{asset.change24h}%
                                </td>
                                <td className="p-4 align-middle [&:has([role=checkbox])]:pr-0 text-right">
                                    ${(asset.volume24h / 1000).toFixed(0)}k
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>
        </div>
    );
};
