import React from 'react';
import { usePagination } from '@/hooks/usePagination';
import { DataTablePagination } from '@/components/ui/DataTablePagination';

interface TopAsset {
  asset: string;
  volume: number;
  tvl: number;
}

interface TopAssetsCardProps {
  assets: TopAsset[];
}

export function TopAssetsCard({ assets }: TopAssetsCardProps) {
  const {
    currentPage,
    pageSize,
    onPageChange,
    onPageSizeChange,
    startIndex,
    endIndex,
  } = usePagination(assets.length, 10);

  const paginatedAssets = assets.slice(startIndex, endIndex);

  return (
    <div className="col-span-1 lg:col-span-2 bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-sm">
      <div className="px-6 py-4 border-b border-slate-800">
        <h2 className="text-sm font-semibold text-white uppercase tracking-wider">Top-performing Assets</h2>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-left text-sm">
          <thead className="text-slate-400 text-xs uppercase bg-slate-950/50">
            <tr>
              <th className="px-6 py-3 font-medium">Asset</th>
              <th className="px-6 py-3 font-medium text-right">Volume</th>
              <th className="px-6 py-3 font-medium text-right">TVL</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800">
            {paginatedAssets.length === 0 ? (
              <tr>
                <td colSpan={3} className="px-6 py-8 text-center text-slate-500">
                  No assets found.
                </td>
              </tr>
            ) : (
              paginatedAssets.map((a) => (
                <tr key={a.asset} className="hover:bg-slate-800/30 transition-colors">
                  <td className="px-6 py-4 font-medium text-white">{a.asset}</td>
                  <td className="px-6 py-4 text-right text-slate-300">{a.volume.toLocaleString()}</td>
                  <td className="px-6 py-4 text-right text-slate-300">${a.tvl.toLocaleString()}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      {assets.length > 0 && (
        <DataTablePagination
          totalItems={assets.length}
          pageSize={pageSize}
          currentPage={currentPage}
          onPageChange={onPageChange}
          onPageSizeChange={onPageSizeChange}
        />
      )}
    </div>
  );
}
