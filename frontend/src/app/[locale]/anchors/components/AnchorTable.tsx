import { Home as AnchorIcon, ExternalLink } from "lucide-react";
import { formatNumber, generateMockHistoricalData, getHealthStatusColor, getHealthStatusIcon, handleSort, SortIndicator, truncateAddress } from "./helpers";
import { Line, LineChart, ResponsiveContainer } from "recharts";
import Link from "next/link";
import { useRouter } from "next/navigation";

const AnchorList = ({
  sortBy,
  sortOrder,
  setSortBy,
  setSortOrder,
  paginatedAnchors
}) => {
  const router = useRouter()
  return (
    <div className="hidden lg:block overflow-x-auto">
      <table className="w-full">
        <thead className="bg-gray-50 dark:bg-slate-700">
          <tr>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
              Anchor
            </th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
              Health Status
            </th>
            <th
              className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-600 select-none"
              onClick={() =>
                handleSort(
                  "reliability",
                  sortBy,
                  sortOrder,
                  setSortBy,
                  setSortOrder,
                )
              }
            >
              <div className="flex items-center gap-1">
                Reliability Score
                <SortIndicator
                  column="reliability"
                  currentSort={sortBy}
                  direction={sortOrder}
                />
              </div>
            </th>
            <th
              className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-600 select-none"
              onClick={() =>
                handleSort(
                  "failure_rate",
                  sortBy,
                  sortOrder,
                  setSortBy,
                  setSortOrder,
                )
              }
            >
              <div className="flex items-center gap-1">
                Success Rate
                <SortIndicator
                  column="failure_rate"
                  currentSort={sortBy}
                  direction={sortOrder}
                />
              </div>
            </th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
              Asset Coverage
            </th>
            <th
              className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-slate-600 select-none"
              onClick={() =>
                handleSort(
                  "transactions",
                  sortBy,
                  sortOrder,
                  setSortBy,
                  setSortOrder,
                )
              }
            >
              <div className="flex items-center gap-1">
                Total Transactions
                <SortIndicator
                  column="transactions"
                  currentSort={sortBy}
                  direction={sortOrder}
                />
              </div>
            </th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
              30-Day Trend
            </th>
            <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
              Actions
            </th>
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-200 dark:divide-slate-700">
          {paginatedAnchors.map((anchor) => {
            const successRate =
              (anchor.successful_transactions /
                anchor.total_transactions) *
              100;
            const historicalData = generateMockHistoricalData(
              anchor.reliability_score,
            );

            return (
              <tr
                key={anchor.id}
                className="hover:bg-gray-50 dark:hover:bg-slate-700 cursor-pointer transition-colors"
                onClick={() =>
                  router.push(`/anchors/${anchor.stellar_account}`)
                }
              >
                <td className="px-6 py-4 whitespace-nowrap">
                  <div className="flex items-center">
                    <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900 rounded-lg flex items-center justify-center mr-3">
                      <AnchorIcon className="w-5 h-5 text-blue-600 dark:text-blue-300" />
                    </div>
                    <div>
                      <div className="text-sm font-medium text-gray-900 dark:text-white">
                        {anchor.name}
                      </div>
                      <div className="text-xs text-gray-500 dark:text-gray-400 font-mono">
                        {truncateAddress(anchor.stellar_account)}
                      </div>
                    </div>
                  </div>
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <span
                    className={`inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium ${getHealthStatusColor(anchor.status)}`}
                  >
                    {getHealthStatusIcon(anchor.status)}
                    {anchor.status}
                  </span>
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <div className="flex items-center">
                    <div className="text-sm font-medium text-gray-900 dark:text-white">
                      {anchor.reliability_score.toFixed(1)}%
                    </div>
                    <div className="ml-2 w-16 bg-gray-200 dark:bg-slate-600 rounded-full h-2">
                      <div
                        className={`h-2 rounded-full ${anchor.reliability_score >= 95
                          ? "bg-green-500"
                          : anchor.reliability_score >= 85
                            ? "bg-yellow-500"
                            : "bg-red-500"
                          }`}
                        style={{
                          width: `${anchor.reliability_score}%`,
                        }}
                      />
                    </div>
                  </div>
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <div className="text-sm text-gray-900 dark:text-white">
                    {successRate.toFixed(1)}%
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400">
                    {formatNumber(anchor.successful_transactions)}/
                    {formatNumber(anchor.total_transactions)}
                  </div>
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <div className="text-sm font-medium text-gray-900 dark:text-white">
                    {anchor.asset_coverage} assets
                  </div>
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <div className="text-sm font-medium text-gray-900 dark:text-white">
                    {formatNumber(anchor.total_transactions)}
                  </div>
                  <div className="text-xs text-red-500">
                    {formatNumber(anchor.failed_transactions)} failed
                  </div>
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <div className="w-20 h-8">
                    <ResponsiveContainer width="100%" height="100%">
                      <LineChart data={historicalData.slice(-7)}>
                        <Line
                          type="monotone"
                          dataKey="score"
                          stroke={
                            anchor.reliability_score >= 95
                              ? "#10b981"
                              : anchor.reliability_score >= 85
                                ? "#f59e0b"
                                : "#ef4444"
                          }
                          strokeWidth={2}
                          dot={false}
                        />
                      </LineChart>
                    </ResponsiveContainer>
                  </div>
                </td>
                <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                  <Link
                    href={`/anchors/${anchor.stellar_account}`}
                    className="text-blue-600 dark:text-blue-400 hover:text-blue-900 dark:hover:text-blue-300 inline-flex items-center gap-1"
                    onClick={(e) => e.stopPropagation()}
                  >
                    View Details
                    <ExternalLink className="w-3 h-3" />
                  </Link>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export default AnchorList;