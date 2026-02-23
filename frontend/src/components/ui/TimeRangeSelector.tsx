import React, { useEffect, useState } from "react";
import { format, subDays, subHours, startOfDay, endOfDay } from "date-fns";

export type Preset = "24h" | "7d" | "30d" | "custom";

interface TimeRangeSelectorProps {
  startDate: Date | null;
  endDate: Date | null;
  onChange: (start: Date | null, end: Date | null) => void;
}

export const TimeRangeSelector: React.FC<TimeRangeSelectorProps> = ({
  startDate,
  endDate,
  onChange,
}) => {
  const [preset, setPreset] = useState<Preset>("30d");

  useEffect(() => {
    // Try to infer a sensible preset from the provided dates.
    if (!startDate || !endDate) return setPreset("custom");
    const now = new Date();
    const diffMs = now.getTime() - startDate.getTime();
    const diffHours = diffMs / (1000 * 60 * 60);

    if (Math.abs(diffHours - 24) < 1) setPreset("24h");
    else if (Math.abs(diffHours - 24 * 7) < 24) setPreset("7d");
    else if (Math.abs(diffHours - 24 * 30) < 48) setPreset("30d");
    else setPreset("custom");
  }, [startDate, endDate]);

  const applyPreset = (p: Preset) => {
    const now = new Date();
    let s: Date | null = null;
    let e: Date | null = now;

    switch (p) {
      case "24h":
        s = subHours(now, 24);
        break;
      case "7d":
        s = startOfDay(subDays(now, 7));
        break;
      case "30d":
        s = startOfDay(subDays(now, 30));
        break;
      case "custom":
        // keep current values
        s = startDate;
        e = endDate;
        break;
    }

    setPreset(p);
    onChange(s, e);
  };

  return (
    <div className="space-y-3">
      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
        Time Range
      </label>

      <div className="flex items-center gap-2">
        {(["24h", "7d", "30d", "custom"] as Preset[]).map((p) => (
          <button
            key={p}
            onClick={() => applyPreset(p)}
            className={`py-1.5 px-3 rounded-lg text-sm font-medium transition ${
              preset === p
                ? "bg-blue-600 text-white"
                : "bg-white dark:bg-slate-800 border border-gray-200 dark:border-slate-700 text-gray-700 dark:text-gray-200"
            }`}
          >
            {p === "24h"
              ? "24h"
              : p === "7d"
                ? "7d"
                : p === "30d"
                  ? "30d"
                  : "Custom"}
          </button>
        ))}
      </div>

      {preset === "custom" && (
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-xs text-gray-500 mb-1">
              Start Date
            </label>
            <input
              type="date"
              className="w-full px-3 py-2 bg-white dark:bg-slate-800 border border-gray-300 dark:border-slate-700 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
              value={startDate ? format(startDate, "yyyy-MM-dd") : ""}
              onChange={(e) => {
                const d = e.target.value ? new Date(e.target.value) : null;
                onChange(d, endDate);
              }}
            />
          </div>
          <div>
            <label className="block text-xs text-gray-500 mb-1">End Date</label>
            <input
              type="date"
              className="w-full px-3 py-2 bg-white dark:bg-slate-800 border border-gray-300 dark:border-slate-700 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
              value={endDate ? format(endDate, "yyyy-MM-dd") : ""}
              onChange={(e) => {
                const d = e.target.value ? new Date(e.target.value) : null;
                onChange(startDate, d);
              }}
            />
          </div>
        </div>
      )}
    </div>
  );
};

export default TimeRangeSelector;
