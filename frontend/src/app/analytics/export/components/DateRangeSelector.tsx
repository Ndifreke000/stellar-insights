import React from "react";
import { format } from "date-fns";

interface DateRangeSelectorProps {
  startDate: Date | null;
  endDate: Date | null;
  onChange: (start: Date | null, end: Date | null) => void;
}

export function DateRangeSelector({
  startDate,
  endDate,
  onChange,
}: DateRangeSelectorProps) {
  return (
    <div className="space-y-4">
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
        Date Range
      </label>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="block text-xs text-gray-500 mb-1">Start Date</label>
          <div className="relative">
            <input
              type="date"
              className="w-full px-3 py-2 bg-white dark:bg-slate-800 border border-gray-300 dark:border-slate-700 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
              value={startDate ? format(startDate, "yyyy-MM-dd") : ""}
              onChange={(e) => {
                const date = e.target.value ? new Date(e.target.value) : null;
                onChange(date, endDate);
              }}
            />
          </div>
        </div>
        <div>
          <label className="block text-xs text-gray-500 mb-1">End Date</label>
          <div className="relative">
            <input
              type="date"
              className="w-full px-3 py-2 bg-white dark:bg-slate-800 border border-gray-300 dark:border-slate-700 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
              value={endDate ? format(endDate, "yyyy-MM-dd") : ""}
              onChange={(e) => {
                const date = e.target.value ? new Date(e.target.value) : null;
                onChange(startDate, date);
              }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
