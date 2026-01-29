"use client";

import React, { useState, useMemo } from "react";
import { MainLayout } from "@/components/layout";
import { ArrowLeft, Download, FileJson, FileText, Mail } from "lucide-react";
import Link from "next/link";
import { DateRangeSelector } from "./components/DateRangeSelector";
import { MetricSelector, MetricOption } from "./components/MetricSelector";
import { ExportPreview } from "./components/ExportPreview";
import { generateCSV, generateJSON, generatePDF } from "@/lib/export-utils";
import { subDays, startOfDay, endOfDay } from "date-fns";

// MOCK DATA GENERATOR
const generateMockData = (start: Date | null, end: Date | null) => {
  if (!start || !end) return [];
  const days =
    Math.floor((end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24)) + 1;
  const data = [];
  for (let i = 0; i < days; i++) {
    const date = new Date(start);
    date.setDate(date.getDate() + i);
    data.push({
      date: date.toISOString(),
      success_rate: 0.95 + (Math.random() * 0.04 - 0.02),
      total_volume: Math.floor(1000000 + Math.random() * 500000),
      active_corridors: Math.floor(20 + Math.random() * 5),
      latency: Math.floor(2000 + Math.random() * 500),
      tvl: Math.floor(5000000 + Math.random() * 1000000),
    });
  }
  return data;
};

export default function ExportPage() {
  const [startDate, setStartDate] = useState<Date | null>(
    subDays(new Date(), 30),
  );
  const [endDate, setEndDate] = useState<Date | null>(new Date());

  const [metrics, setMetrics] = useState<MetricOption[]>([
    { id: "date", label: "Date/Time", checked: true },
    { id: "success_rate", label: "Success Rate", checked: true },
    { id: "total_volume", label: "Total Volume (USD)", checked: true },
    { id: "active_corridors", label: "Active Corridors", checked: false },
    { id: "latency", label: "Avg Latency (ms)", checked: false },
    { id: "tvl", label: "Total Value Locked", checked: true },
  ]);

  // Generate preview data based on date range
  const previewData = useMemo(() => {
    const start = startDate ? startOfDay(startDate) : null;
    const end = endDate ? endOfDay(endDate) : null;
    return generateMockData(start, end);
  }, [startDate, endDate]);

  const handleMetricChange = (id: string, checked: boolean) => {
    setMetrics((prev) =>
      prev.map((m) => (m.id === id ? { ...m, checked } : m)),
    );
  };

  const getActiveColumns = () =>
    metrics.filter((m) => m.checked).map((m) => ({ id: m.id, label: m.label }));

  const handleExport = (format: "csv" | "json" | "pdf") => {
    const columns = getActiveColumns();
    if (columns.length === 0) {
      alert("Please select at least one metric to export.");
      return;
    }

    switch (format) {
      case "csv":
        generateCSV(previewData, columns);
        break;
      case "json":
        generateJSON(previewData, columns);
        break;
      case "pdf":
        generatePDF(previewData, columns, { start: startDate, end: endDate });
        break;
    }
  };

  return (
    <MainLayout>
      <div className="p-4 sm:p-6 lg:p-8 max-w-7xl mx-auto min-h-screen flex flex-col">
        {/* Header */}
        <div className="mb-8 flex items-center gap-4">
          <Link
            href="/analytics"
            className="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-slate-800 transition-colors"
          >
            <ArrowLeft className="w-5 h-5 text-gray-600 dark:text-gray-400" />
          </Link>
          <div>
            <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
              Export Data
            </h1>
            <p className="text-sm text-gray-500 dark:text-gray-400">
              Download custom reports
            </p>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 flex-1">
          {/* Left Column: Controls */}
          <div className="lg:col-span-4 space-y-8">
            <div className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm space-y-6">
              <DateRangeSelector
                startDate={startDate}
                endDate={endDate}
                onChange={(s, e) => {
                  setStartDate(s);
                  setEndDate(e);
                }}
              />

              <hr className="border-gray-100 dark:border-slate-700" />

              <MetricSelector metrics={metrics} onChange={handleMetricChange} />
            </div>

            {/* Export Actions */}
            <div className="bg-white dark:bg-slate-800 p-6 rounded-xl border border-gray-200 dark:border-slate-700 shadow-sm">
              <h3 className="text-sm font-medium text-gray-900 dark:text-white mb-4">
                Export As
              </h3>
              <div className="grid grid-cols-1 gap-3">
                <button
                  onClick={() => handleExport("csv")}
                  className="flex items-center justify-center gap-2 w-full py-2.5 px-4 bg-white dark:bg-slate-800 border border-gray-300 dark:border-slate-600 rounded-lg hover:bg-gray-50 dark:hover:bg-slate-700 transition font-medium text-sm text-gray-700 dark:text-gray-200"
                >
                  <FileText className="w-4 h-4 text-green-600" />
                  Download CSV
                </button>
                <button
                  onClick={() => handleExport("json")}
                  className="flex items-center justify-center gap-2 w-full py-2.5 px-4 bg-white dark:bg-slate-800 border border-gray-300 dark:border-slate-600 rounded-lg hover:bg-gray-50 dark:hover:bg-slate-700 transition font-medium text-sm text-gray-700 dark:text-gray-200"
                >
                  <FileJson className="w-4 h-4 text-orange-500" />
                  Download JSON
                </button>
                <button
                  onClick={() => handleExport("pdf")}
                  className="flex items-center justify-center gap-2 w-full py-2.5 px-4 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition font-medium text-sm shadow-sm"
                >
                  <Download className="w-4 h-4" />
                  Generate PDF Report
                </button>
              </div>

              <div className="mt-4 pt-4 border-t border-gray-100 dark:border-slate-700">
                <button className="flex items-center justify-center gap-2 w-full py-2 text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 transition">
                  <Mail className="w-4 h-4" />
                  Email Report
                </button>
              </div>
            </div>
          </div>

          {/* Right Column: Preview */}
          <div className="lg:col-span-8 h-full min-h-[500px]">
            <ExportPreview metrics={metrics} data={previewData} />
          </div>
        </div>
      </div>
    </MainLayout>
  );
}
