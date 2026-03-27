import { formatAddressShort } from "@/lib/address";
import { Activity, AlertCircle, CheckCircle } from "lucide-react";
import {
  Search,
  TrendingUp,
  TrendingDown,
  Download,
} from "lucide-react"

const truncateAddress = (address: string) => formatAddressShort(address, 6, 4);



const generateMockHistoricalData = (currentScore: number) => {
  const data = [];
  for (let i = 30; i >= 0; i--) {
    const variation = (Math.random() - 0.5) * 10; // ±5 point variation
    const score = Math.max(0, Math.min(100, currentScore + variation));
    data.push({
      date: new Date(Date.now() - i * 24 * 60 * 60 * 1000)
        .toISOString()
        .split("T")[0],
      score: score,
    });
  }
  return data;
};

// Sort handler function
const handleSort = (
  column: "reliability" | "transactions" | "failure_rate",
  currentSortBy: string,
  currentDirection: "asc" | "desc",
  setSortBy: (sort: "reliability" | "transactions" | "failure_rate") => void,
  setSortDirection: (dir: "asc" | "desc") => void,
) => {
  if (currentSortBy === column) {
    // Toggle direction if same column
    setSortDirection(currentDirection === "asc" ? "desc" : "asc");
  } else {
    // New column, default to desc for most metrics
    setSortBy(column);
    setSortDirection(column === "failure_rate" ? "asc" : "desc");
  }
};

// Sort indicator component
const SortIndicator = ({
  column,
  currentSort,
  direction,
}: {
  column: string;
  currentSort: string;
  direction: "asc" | "desc";
}) => {
  if (currentSort !== column) {
    return <span className="text-muted-foreground w-4 h-4 inline-block text-center">↕</span>;
  }
  return direction === "asc" ? (
    <span className="text-blue-500 w-4 h-4 inline-block text-center">↑</span>
  ) : (
    <span className="text-blue-500 w-4 h-4 inline-block text-center">↓</span>
  );
};
const getHealthStatusColor = (status: string) => {
  switch (status.toLowerCase()) {
    case "healthy":
      return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300";
    case "warning":
      return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300";
    case "critical":
      return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300";
    default:
      return "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-300";
  }
};

const getHealthStatusIcon = (status: string) => {
  switch (status.toLowerCase()) {
    case "healthy":
      return <CheckCircle className="w-4 h-4" />;
    case "warning":
      return <AlertCircle className="w-4 h-4" />;
    case "critical":
      return <AlertCircle className="w-4 h-4" />;
    default:
      return <Activity className="w-4 h-4" />;
  }
};

const formatNumber = (num: number) => {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
};

const Error = ({error}:{error?:string}) => {
  return (
    <div className="mb-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
      <div className="flex items-center gap-2">
        <div className="text-red-600 dark:text-red-400 font-medium">
          Error loading anchors
        </div>
      </div>
      <div className="text-sm text-red-600 dark:text-red-400 mt-1">
        {error}
      </div>
    </div>
  )
}

const SearchAndControls = ({ searchTerm, setSearchTerm, sortBy, setSortBy, setSortOrder, sortOrder, setIsExportOpen }) => {
  return (
    <div className="mb-6 flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between">
      <div className="flex-1 relative w-full sm:max-w-md">
        <Search className="absolute left-3 top-2.5 w-5 h-5 text-muted-foreground" />
        <input
          type="text"
          placeholder="Search anchors by name or account..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="w-full pl-10 pr-4 py-2 border border-gray-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Sort Controls */}
      <div className="flex gap-2">
        <select
          value={sortBy}
          onChange={(e) =>
            setSortBy(
              e.target.value as
              | "reliability"
              | "transactions"
              | "failure_rate",
            )
          }
          className="px-3 py-2 border border-gray-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="reliability">Reliability Score</option>
          <option value="transactions">Total Transactions</option>
          <option value="failure_rate">Failure Rate</option>
        </select>

        <button
          onClick={() =>
            setSortOrder(sortOrder === "desc" ? "asc" : "desc")
          }
          className="px-3 py-2 border border-gray-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-gray-900 dark:text-white hover:bg-gray-50 dark:hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          {sortOrder === "desc" ? (
            <TrendingDown className="w-4 h-4" />
          ) : (
            <TrendingUp className="w-4 h-4" />
          )}
        </button>
        <button
          onClick={() => setIsExportOpen(true)}
          className="flex items-center gap-2 px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg text-sm font-medium transition-colors shadow-sm"
        >
          <Download className="w-4 h-4" />
          Export
        </button>
      </div>
    </div>
  )
}

export {
  truncateAddress,
  getHealthStatusColor,
  getHealthStatusIcon,
  generateMockHistoricalData,
  handleSort,
  SortIndicator,
  formatNumber,
  Error,
  SearchAndControls
}

