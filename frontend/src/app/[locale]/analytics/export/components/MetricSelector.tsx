import React from "react";
import { Check } from "lucide-react";

export type MetricOption = {
  id: string;
  label: string;
  checked: boolean;
};

interface MetricSelectorProps {
  metrics: MetricOption[];
  onChange: (id: string, checked: boolean) => void;
}

export function MetricSelector({ metrics, onChange }: MetricSelectorProps) {
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
          Select Metrics
        </label>
        <button
          className="text-xs text-blue-600 hover:text-blue-700 font-medium"
          onClick={() => metrics.forEach((m) => onChange(m.id, true))}
        >
          Select All
        </button>
      </div>

      <div className="space-y-2 max-h-[300px] overflow-y-auto pr-1">
        {metrics.map((metric) => (
          <label
            key={metric.id}
            className={`flex items-center justify-between p-3 rounded-lg border cursor-pointer transition-all ${
              metric.checked
                ? "bg-blue-50 border-blue-200 dark:bg-blue-900/20 dark:border-blue-800"
                : "bg-white border-gray-200 hover:border-blue-300 dark:bg-slate-800 dark:border-slate-700"
            }`}
          >
            <span className="text-sm text-gray-700 dark:text-gray-300 font-medium">
              {metric.label}
            </span>
            <div
              className={`w-5 h-5 rounded border flex items-center justify-center transition-colors ${
                metric.checked
                  ? "bg-blue-500 border-blue-500 text-white"
                  : "bg-white border-gray-400 dark:bg-slate-700 dark:border-slate-600"
              }`}
            >
              <input
                type="checkbox"
                className="hidden"
                checked={metric.checked}
                onChange={(e) => onChange(metric.id, e.target.checked)}
              />
              {metric.checked && <Check className="w-3.5 h-3.5" />}
            </div>
          </label>
        ))}
      </div>
    </div>
  );
}
