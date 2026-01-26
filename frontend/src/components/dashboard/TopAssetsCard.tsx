import React from 'react';

interface TopAsset {
  asset: string;
  volume: number;
  tvl: number;
}

interface TopAssetsCardProps {
  assets: TopAsset[];
}

export function TopAssetsCard({ assets }: TopAssetsCardProps) {
  return (
    <div className="col-span-1 lg:col-span-2 bg-white rounded shadow p-4">
      <h2 className="text-sm text-gray-500">Top-performing Assets</h2>
      <div className="mt-3 overflow-auto">
        <table className="w-full text-left text-sm">
          <thead className="text-gray-500 text-xs uppercase">
            <tr>
              <th className="pb-2">Asset</th>
              <th className="pb-2">Volume</th>
              <th className="pb-2">TVL</th>
            </tr>
          </thead>
          <tbody>
            {assets.map((a) => (
              <tr key={a.asset} className="border-t">
                <td className="py-2 font-medium">{a.asset}</td>
                <td className="py-2">{a.volume.toLocaleString()}</td>
                <td className="py-2">${a.tvl.toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
