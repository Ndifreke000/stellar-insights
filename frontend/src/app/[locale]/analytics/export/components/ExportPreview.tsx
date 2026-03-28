import React from "react";
import { MetricOption } from "./MetricSelector";
import { format } from "date-fns";

interface ExportDataRow {
  [key: string]: string | number | boolean | null;
}

interface ExportPreviewProps {
  metrics: MetricOption[];
  data: ExportDataRow[];
}

export function ExportPreview({ metrics, data }: ExportPreviewProps) {
  const activeMetrics = metrics.filter((m) => m.checked);

  if (activeMetrics.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center p-12 text-center text-muted-foreground border-2 border-dashed border-gray-200 dark:border-slate-700 rounded-xl h-full">
        <p>Select at least one metric to see preview</p>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-slate-800 rounded-xl border border-gray-200 dark:border-slate-700 overflow-hidden shadow-sm flex flex-col h-full">
      <div className="p-4 border-b border-gray-200 dark:border-slate-700 bg-gray-50 dark:bg-slate-800">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-white">
          Preview
        </h3>
        <p className="text-xs text-muted-foreground">First 5 rows of export</p>
      </div>
      <div className="overflow-x-auto flex-1">
        <table className="w-full text-sm text-left">
          <thead className="bg-gray-50 dark:bg-slate-700/50 text-gray-700 dark:text-gray-300">
            <tr>
              {activeMetrics.map((metric) => (
                <th
                  key={metric.id}
                  className="px-6 py-3 font-medium whitespace-nowrap"
                >
                  {metric.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-200 dark:divide-slate-700">
            {data.slice(0, 5).map((row, idx) => (
              <tr
                key={idx}
                className="hover:bg-gray-50 dark:hover:bg-slate-700/30 transition-colors"
              >
                {activeMetrics.map((metric) => (
                  <td
                    key={`${idx}-${metric.id}`}
                    className="px-6 py-4 whitespace-nowrap text-muted-foreground dark:text-gray-300"
                  >
                    {(() => {
                      const value = row[metric.id];
                      if (metric.id === "date" && value != null) {
                        return format(new Date(String(value)), "yyyy-MM-dd HH:mm");
                      }
                      if (metric.id === "success_rate" && typeof value === "number") {
                        return `${(value * 100).toFixed(2)}%`;
                      }
                      if (metric.id === "total_volume" && typeof value === "number") {
                        return `$${value.toLocaleString()}`;
                      }
                      if (metric.id === "tvl" && typeof value === "number") {
                        return `$${value.toLocaleString()}`;
                      }
                      if (metric.id === "latency") {
                        return `${value} ms`;
                      }
                      return String(value ?? "");
                    })()}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="p-3 border-t border-gray-200 dark:border-slate-700 text-xs text-center text-muted-foreground bg-gray-50 dark:bg-slate-800">
        Showing 5 of {data.length} records
      </div>
    </div>
  );
}
