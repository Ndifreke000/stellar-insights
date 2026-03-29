import { formatNumber, generateMockHistoricalData, getHealthStatusColor, getHealthStatusIcon, truncateAddress } from "./helpers";
import { Line, LineChart, ResponsiveContainer } from "recharts";
import { Home as AnchorIcon, BarChart3, ExternalLink } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
const AnchorCards = ({
  paginatedAnchors
}) => {
  const router = useRouter()
  return (
    <div className="lg:hidden divide-y divide-gray-200 dark:divide-slate-700">
      {paginatedAnchors.map((anchor) => {
        const successRate =
          (anchor.successful_transactions / anchor.total_transactions) *
          100;
        const historicalData = generateMockHistoricalData(
          anchor.reliability_score,
        );

        return (
          <article
            key={anchor.id}
            className="p-4 cursor-pointer hover:bg-gray-50 dark:hover:bg-slate-700 transition-colors"
            role="button"
            tabIndex={0}
            aria-label={`Open anchor details for ${anchor.name}`}
            onClick={() => router.push(`/anchors/${anchor.stellar_account}`)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                router.push(`/anchors/${anchor.stellar_account}`);
              }
            }}
          >
            <div className="flex items-start justify-between mb-3">
              <div className="flex items-center">
                <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900 rounded-lg flex items-center justify-center mr-3">
                  <AnchorIcon aria-hidden="true" className="w-5 h-5 text-blue-600 dark:text-blue-300" />
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
              <span
                className={`inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium ${getHealthStatusColor(anchor.status)}`}
              >
                {getHealthStatusIcon(anchor.status)}
                {anchor.status}
              </span>
            </div>

            <div className="grid grid-cols-2 gap-4 mb-3">
              <div>
                <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Reliability
                </div>
                <div className="flex items-center">
                  <span className="text-sm font-medium text-gray-900 dark:text-white mr-2">
                    {anchor.reliability_score.toFixed(1)}%
                  </span>
                  <div className="flex-1 bg-gray-200 dark:bg-slate-600 rounded-full h-2">
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
              </div>
              <div>
                <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Success Rate
                </div>
                <div className="text-sm font-medium text-gray-900 dark:text-white">
                  {successRate.toFixed(1)}%
                </div>
              </div>
              <div>
                <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Assets
                </div>
                <div className="text-sm font-medium text-gray-900 dark:text-white">
                  {anchor.asset_coverage}
                </div>
              </div>
              <div>
                <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Transactions
                </div>
                <div className="text-sm font-medium text-gray-900 dark:text-white">
                  {formatNumber(anchor.total_transactions)}
                </div>
              </div>
            </div>

            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <BarChart3 aria-hidden="true" className="w-4 h-4 text-gray-400" />
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  30-day trend
                </span>
                <div className="w-16 h-6">
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
              </div>
              <Link
                href={`/anchors/${anchor.stellar_account}`}
                className="text-blue-600 dark:text-blue-400 hover:text-blue-900 dark:hover:text-blue-300 inline-flex items-center gap-1 text-sm"
                onClick={(e) => e.stopPropagation()}
              >
                Details
                <ExternalLink className="w-3 h-3" />
              </Link>
            </div>
          </div>
        );
      })}
    </div>
  );
}

export default AnchorCards;