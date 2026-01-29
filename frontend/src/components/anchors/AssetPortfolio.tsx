"use client";

import { useState, useMemo } from "react";
import { IssuedAsset } from "@/lib/api";
import { IssuedAssetsTable } from "./IssuedAssetsTable";
import { AssetDistributionChart } from "./AssetDistributionChart";
import { AssetDetailModal } from "./AssetDetailModal";
import { Download, Filter, ArrowUpDown } from "lucide-react";

interface AssetPortfolioProps {
  assets: IssuedAsset[];
}

type SortKey = "volume_24h_usd" | "success_rate" | "total_transactions";
type SortDirection = "asc" | "desc";

export function AssetPortfolio({ assets }: AssetPortfolioProps) {
  const [selectedAsset, setSelectedAsset] = useState<IssuedAsset | null>(null);
  const [filterType, setFilterType] = useState<
    "all" | "credit_alphanum4" | "credit_alphanum12"
  >("all");
  const [sortConfig, setSortConfig] = useState<{
    key: SortKey;
    direction: SortDirection;
  }>({
    key: "volume_24h_usd",
    direction: "desc",
  });

  const filteredAndSortedAssets = useMemo(() => {
    let result = [...assets];

    // Filter
    if (filterType !== "all") {
      result = result.filter((asset) => {
        const codeLength = asset.asset_code.length;
        if (filterType === "credit_alphanum4") return codeLength <= 4;
        if (filterType === "credit_alphanum12")
          return codeLength > 4 && codeLength <= 12;
        return true;
      });
    }

    // Sort
    result.sort((a, b) => {
      const aValue = a[sortConfig.key];
      const bValue = b[sortConfig.key];

      if (aValue < bValue) return sortConfig.direction === "asc" ? -1 : 1;
      if (aValue > bValue) return sortConfig.direction === "asc" ? 1 : -1;
      return 0;
    });

    return result;
  }, [assets, filterType, sortConfig]);

  const handleExport = () => {
    const headers = [
      "Asset Code",
      "Issuer",
      "Volume (24h USD)",
      "Success Rate (%)",
      "Failure Rate (%)",
      "Transactions",
    ];
    const csvContent = [
      headers.join(","),
      ...filteredAndSortedAssets.map((asset) =>
        [
          asset.asset_code,
          asset.issuer,
          asset.volume_24h_usd,
          asset.success_rate.toFixed(2),
          asset.failure_rate.toFixed(2),
          asset.total_transactions,
        ].join(","),
      ),
    ].join("\n");

    const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.setAttribute("href", url);
    link.setAttribute(
      "download",
      `anchor_assets_${new Date().toISOString().split("T")[0]}.csv`,
    );
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const toggleSort = (key: SortKey) => {
    setSortConfig((current) => ({
      key,
      direction:
        current.key === key && current.direction === "desc" ? "asc" : "desc",
    }));
  };

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Left: Chart */}
        <div className="lg:col-span-1 h-full min-h-[400px]">
          <AssetDistributionChart assets={filteredAndSortedAssets} />
        </div>

        {/* Right: Table & Controls */}
        <div className="lg:col-span-2 space-y-4 flex flex-col h-full">
          {/* Controls */}
          <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4 bg-slate-900 border border-slate-800 p-4 rounded-xl">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <Filter className="w-4 h-4 text-slate-400" />
                <select
                  className="bg-slate-950 border border-slate-700 text-slate-300 text-sm rounded-lg focus:ring-indigo-500 focus:border-indigo-500 block p-2"
                  value={filterType}
                  onChange={(e) => setFilterType(e.target.value as any)}
                >
                  <option value="all">All Types</option>
                  <option value="credit_alphanum4">Alphanum 4</option>
                  <option value="credit_alphanum12">Alphanum 12</option>
                </select>
              </div>

              <div className="h-6 w-px bg-slate-800 hidden sm:block"></div>

              <div className="flex items-center gap-2">
                <ArrowUpDown className="w-4 h-4 text-slate-400" />
                <span className="text-sm text-slate-400">Sort:</span>
                <div className="flex gap-1">
                  <button
                    onClick={() => toggleSort("volume_24h_usd")}
                    className={`px-2 py-1 text-xs rounded-md ${sortConfig.key === "volume_24h_usd" ? "bg-indigo-500/20 text-indigo-400" : "text-slate-500 hover:text-slate-300"}`}
                  >
                    Vol
                  </button>
                  <button
                    onClick={() => toggleSort("success_rate")}
                    className={`px-2 py-1 text-xs rounded-md ${sortConfig.key === "success_rate" ? "bg-indigo-500/20 text-indigo-400" : "text-slate-500 hover:text-slate-300"}`}
                  >
                    Success
                  </button>
                </div>
              </div>
            </div>

            <button
              onClick={handleExport}
              className="flex items-center gap-2 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-white rounded-lg transition-colors text-sm font-medium"
            >
              <Download className="w-4 h-4" />
              Export CSV
            </button>
          </div>

          {/* Table */}
          <div className="flex-1 min-h-0">
            <IssuedAssetsTable
              assets={filteredAndSortedAssets}
              onAssetClick={setSelectedAsset}
            />
          </div>
        </div>
      </div>

      <AssetDetailModal
        asset={selectedAsset}
        onClose={() => setSelectedAsset(null)}
      />
    </div>
  );
}
