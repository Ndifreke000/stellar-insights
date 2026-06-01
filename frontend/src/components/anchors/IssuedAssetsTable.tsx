import { IssuedAsset } from "@/lib/api";
import { ArrowUpRight, AlertTriangle } from "lucide-react";
import { usePagination } from "@/hooks/usePagination";
import { DataTablePagination } from "@/components/ui/DataTablePagination";

interface IssuedAssetsTableProps {
  assets: IssuedAsset[];
}

export function IssuedAssetsTable({
  assets,
  onAssetClick,
}: IssuedAssetsTableProps & { onAssetClick?: (asset: IssuedAsset) => void }) {
  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
      maximumFractionDigits: 0,
      notation: "compact",
    }).format(value);
  };

  const formatPercent = (value: number) => {
    return `${value.toFixed(1)}% `;
  };

  const {
    currentPage,
    pageSize,
    onPageChange,
    onPageSizeChange,
    startIndex,
    endIndex,
  } = usePagination(assets.length);

  const paginatedAssets = assets.slice(startIndex, endIndex);

  return (
    <div className="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-sm h-full flex flex-col">
      <div className="px-6 py-4 border-b border-slate-800 flex justify-between items-center">
        <h3 className="font-semibold text-white">Issued Assets</h3>
        <span className="text-xs text-slate-500 uppercase tracking-wider font-mono">
          {assets.length} Total
        </span>
      </div>

      <div className="overflow-x-auto flex-1">
        <table className="w-full text-sm text-left">
          <thead className="text-xs text-slate-400 uppercase bg-slate-950/50">
            <tr>
              <th className="px-6 py-3 font-medium">Asset</th>
              <th className="px-6 py-3 font-medium text-right">Volume (24h)</th>
              <th className="px-6 py-3 font-medium text-right">Success Rate</th>
              <th className="px-6 py-3 font-medium text-right">Failure Rate</th>
              <th className="px-6 py-3 font-medium text-right">Transactions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800">
            {assets.length === 0 ? (
              <tr>
                <td
                  colSpan={5}
                  className="px-6 py-8 text-center text-slate-500"
                >
                  No assets found for this anchor.
                </td>
              </tr>
            ) : (
              paginatedAssets.map((asset) => (
                <tr
                  key={asset.asset_code}
                  className={`transition-colors ${onAssetClick ? "cursor-pointer hover:bg-slate-800/50" : "hover:bg-slate-800/30"}`}
                  onClick={() => onAssetClick?.(asset)}
                >
                  <td className="px-6 py-4 font-medium text-white">
                    <div className="flex items-center gap-2">
                      <div className="w-8 h-8 rounded-full bg-indigo-500/10 flex items-center justify-center text-indigo-400 border border-indigo-500/20 font-bold text-xs">
                        {asset.asset_code.substring(0, 2)}
                      </div>
                      <span>{asset.asset_code}</span>
                    </div>
                  </td>
                  <td className="px-6 py-4 text-right text-slate-300">
                    {formatCurrency(asset.volume_24h_usd)}
                  </td>
                  <td className="px-6 py-4 text-right">
                    <div className="flex items-center justify-end gap-1.5 text-emerald-400">
                      <ArrowUpRight className="w-3.5 h-3.5" />
                      {formatPercent(asset.success_rate)}
                    </div>
                  </td>
                  <td className="px-6 py-4 text-right">
                    <div
                      className={`flex items-center justify-end gap-1.5 ${
                        asset.failure_rate > 5
                          ? "text-rose-400 font-medium"
                          : "text-slate-400"
                      }`}
                    >
                      {asset.failure_rate > 5 && (
                        <AlertTriangle className="w-3.5 h-3.5" />
                      )}
                      {formatPercent(asset.failure_rate)}
                    </div>
                  </td>
                  <td className="px-6 py-4 text-right text-slate-400 font-mono">
                    {asset.total_transactions.toLocaleString()}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      {assets.length > 0 && (
        <div className="mt-auto">
          <DataTablePagination
            totalItems={assets.length}
            pageSize={pageSize}
            currentPage={currentPage}
            onPageChange={onPageChange}
            onPageSizeChange={onPageSizeChange}
          />
        </div>
      )}
    </div>
  );
}
