"use client";

import React, { useState, useEffect } from "react";
import {
  getExportRequests,
  getExportableTypes,
  createExportRequest,
  ExportRequestResponse,
  ExportableDataTypes,
} from "@/lib/gdpr-api";

interface DataExportProps {
  onClose?: () => void;
}

export function DataExport({ onClose }: DataExportProps) {
  const [exportRequests, setExportRequests] = useState<ExportRequestResponse[]>(
    [],
  );
  const [availableTypes, setAvailableTypes] =
    useState<ExportableDataTypes | null>(null);
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"new" | "history">("new");

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);

      const [requests, types] = await Promise.all([
        getExportRequests(),
        getExportableTypes(),
      ]);

      setExportRequests(requests);
      setAvailableTypes(types);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load data");
    } finally {
      setLoading(false);
    }
  };

  const handleTypeToggle = (typeId: string) => {
    setSelectedTypes((prev) =>
      prev.includes(typeId)
        ? prev.filter((t) => t !== typeId)
        : [...prev, typeId],
    );
  };

  const handleSelectAll = () => {
    if (availableTypes) {
      setSelectedTypes(availableTypes.types.map((t) => t.id));
    }
  };

  const handleDeselectAll = () => {
    setSelectedTypes([]);
  };

  const handleCreateExport = async () => {
    if (selectedTypes.length === 0) {
      setError("Please select at least one data type to export");
      return;
    }

    try {
      setCreating(true);
      setError(null);
      setSuccess(null);

      const result = await createExportRequest({
        data_types: selectedTypes,
        export_format: "json",
      });

      setSuccess(`Export request created! Your data will be ready shortly.`);
      setSelectedTypes([]);
      await loadData();
      setActiveTab("history");
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to create export request",
      );
    } finally {
      setCreating(false);
    }
  };

  const getStatusBadge = (status: string) => {
    const statusStyles: Record<string, string> = {
      pending:
        "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400",
      processing:
        "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-link-primary",
      completed:
        "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400",
      expired:
        "bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-muted-foreground",
      failed: "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400",
    };

    return (
      <span
        className={`px-2 py-1 text-xs font-medium rounded-full ${
          statusStyles[status] || statusStyles.pending
        }`}
      >
        {status.charAt(0).toUpperCase() + status.slice(1)}
      </span>
    );
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
          Export Your Data
        </h2>
        {onClose && (
          <button
            onClick={onClose}
            className="text-muted-foreground hover:text-gray-700 dark:text-muted-foreground dark:hover:text-gray-200"
          >
            <svg
              className="w-6 h-6"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        )}
      </div>

      {/* Tabs */}
      <div className="flex border-b border-gray-200 dark:border-gray-700 mb-6">
        <button
          onClick={() => setActiveTab("new")}
          className={`px-4 py-2 font-medium text-sm ${
            activeTab === "new"
              ? "border-b-2 border-blue-600 text-blue-600 dark:text-link-primary"
              : "text-muted-foreground dark:text-muted-foreground hover:text-gray-700 dark:hover:text-gray-200"
          }`}
        >
          New Export
        </button>
        <button
          onClick={() => setActiveTab("history")}
          className={`px-4 py-2 font-medium text-sm ${
            activeTab === "history"
              ? "border-b-2 border-blue-600 text-blue-600 dark:text-link-primary"
              : "text-muted-foreground dark:text-muted-foreground hover:text-gray-700 dark:hover:text-gray-200"
          }`}
        >
          Export History
        </button>
      </div>

      {error && (
        <div className="mb-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
          <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
        </div>
      )}

      {success && (
        <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-md">
          <p className="text-sm text-green-600 dark:text-green-400">
            {success}
          </p>
        </div>
      )}

      {activeTab === "new" && availableTypes && (
        <div>
          <div className="flex justify-between items-center mb-4">
            <p className="text-sm text-muted-foreground dark:text-muted-foreground">
              Select the data you want to export:
            </p>
            <div className="flex gap-2">
              <button
                onClick={handleSelectAll}
                className="text-xs text-blue-600 hover:text-blue-700 dark:text-link-primary"
              >
                Select All
              </button>
              <span className="text-gray-300 dark:text-muted-foreground">|</span>
              <button
                onClick={handleDeselectAll}
                className="text-xs text-muted-foreground hover:text-gray-700 dark:text-muted-foreground"
              >
                Deselect All
              </button>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mb-6">
            {availableTypes.types.map((type) => (
              <label
                key={type.id}
                className={`flex items-start p-3 border rounded-lg cursor-pointer transition-colors ${
                  selectedTypes.includes(type.id)
                    ? "border-blue-500 bg-blue-50 dark:bg-blue-900/20"
                    : "border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600"
                }`}
              >
                <input
                  type="checkbox"
                  checked={selectedTypes.includes(type.id)}
                  onChange={() => handleTypeToggle(type.id)}
                  className="mt-1 mr-3"
                />
                <div>
                  <p className="font-medium text-gray-900 dark:text-white text-sm">
                    {type.name}
                  </p>
                  <p className="text-xs text-muted-foreground dark:text-muted-foreground mt-1">
                    {type.description}
                  </p>
                </div>
              </label>
            ))}
          </div>

          <button
            onClick={handleCreateExport}
            disabled={creating || selectedTypes.length === 0}
            className="w-full px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {creating
              ? "Creating Export..."
              : `Export ${selectedTypes.length} Data Type(s)`}
          </button>
        </div>
      )}

      {activeTab === "history" && (
        <div>
          {exportRequests.length === 0 ? (
            <p className="text-center text-muted-foreground dark:text-muted-foreground py-8">
              No export requests yet.
            </p>
          ) : (
            <div className="space-y-3">
              {exportRequests.map((request) => (
                <div
                  key={request.id}
                  className="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg"
                >
                  <div>
                    <p className="text-sm text-gray-900 dark:text-white">
                      Request #{request.id.slice(0, 8)}
                    </p>
                    <p className="text-xs text-muted-foreground dark:text-muted-foreground mt-1">
                      Created: {new Date(request.requested_at).toLocaleString()}
                    </p>
                    {request.expires_at && (
                      <p className="text-xs text-muted-foreground dark:text-muted-foreground">
                        Expires: {new Date(request.expires_at).toLocaleString()}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-3">
                    {getStatusBadge(request.status)}
                    {request.download_url && request.status === "completed" && (
                      <a
                        href={request.download_url}
                        className="px-3 py-1 text-sm bg-green-600 text-white rounded hover:bg-green-700 transition-colors"
                      >
                        Download
                      </a>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default DataExport;
