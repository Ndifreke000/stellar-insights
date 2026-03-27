"use client";

import React, { useState, useEffect } from "react";
import { ConsentManager, DataExport, DataDeletion } from "@/components/gdpr";
import { getGdprSummary, GdprSummary } from "@/lib/gdpr-api";

type TabType = "overview" | "consents" | "export" | "deletion";

export default function GdprSettingsPage() {
  const [activeTab, setActiveTab] = useState<TabType>("overview");
  const [summary, setSummary] = useState<GdprSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadSummary();
  }, []);

  const loadSummary = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await getGdprSummary();
      setSummary(data);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to load GDPR summary",
      );
    } finally {
      setLoading(false);
    }
  };

  const renderOverview = () => (
    <div className="space-y-6">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Your Data Rights
        </h3>
        <p className="text-muted-foreground dark:text-muted-foreground mb-6">
          Under the General Data Protection Regulation (GDPR), you have certain
          rights regarding your personal data. This page allows you to manage
          your data in accordance with these rights.
        </p>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {/* Privacy Preferences Card */}
          <button
            onClick={() => setActiveTab("consents")}
            className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg text-left hover:border-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors"
          >
            <div className="flex items-center mb-2">
              <div className="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-full flex items-center justify-center mr-3">
                <svg
                  className="w-5 h-5 text-blue-600 dark:text-link-primary"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"
                  />
                </svg>
              </div>
              <div>
                <h4 className="font-medium text-gray-900 dark:text-white">
                  Privacy Preferences
                </h4>
                <p className="text-xs text-muted-foreground dark:text-muted-foreground">
                  Manage your consent settings
                </p>
              </div>
            </div>
            {summary && (
              <p className="text-sm text-muted-foreground dark:text-muted-foreground">
                {summary.consents.filter((c) => c.consent_given).length} of{" "}
                {summary.consents.length} consents granted
              </p>
            )}
          </button>

          {/* Export Data Card */}
          <button
            onClick={() => setActiveTab("export")}
            className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg text-left hover:border-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors"
          >
            <div className="flex items-center mb-2">
              <div className="w-10 h-10 bg-green-100 dark:bg-green-900/30 rounded-full flex items-center justify-center mr-3">
                <svg
                  className="w-5 h-5 text-green-600 dark:text-green-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                  />
                </svg>
              </div>
              <div>
                <h4 className="font-medium text-gray-900 dark:text-white">
                  Export Your Data
                </h4>
                <p className="text-xs text-muted-foreground dark:text-muted-foreground">
                  Download a copy of your data
                </p>
              </div>
            </div>
            {summary && summary.pending_export_requests > 0 && (
              <p className="text-sm text-orange-600 dark:text-orange-400">
                {summary.pending_export_requests} pending request(s)
              </p>
            )}
          </button>

          {/* Delete Data Card */}
          <button
            onClick={() => setActiveTab("deletion")}
            className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg text-left hover:border-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
          >
            <div className="flex items-center mb-2">
              <div className="w-10 h-10 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center mr-3">
                <svg
                  className="w-5 h-5 text-red-600 dark:text-red-400"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                  />
                </svg>
              </div>
              <div>
                <h4 className="font-medium text-gray-900 dark:text-white">
                  Delete Your Data
                </h4>
                <p className="text-xs text-muted-foreground dark:text-muted-foreground">
                  Request data deletion
                </p>
              </div>
            </div>
            {summary && summary.pending_deletion_requests > 0 && (
              <p className="text-sm text-orange-600 dark:text-orange-400">
                {summary.pending_deletion_requests} pending request(s)
              </p>
            )}
          </button>
        </div>
      </div>

      {/* Data Processing Info */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Data Processing Activities
        </h3>
        <p className="text-muted-foreground dark:text-muted-foreground">
          We have recorded{" "}
          <span className="font-medium text-gray-900 dark:text-white">
            {summary?.data_processing_activities_count || 0}
          </span>{" "}
          data processing activities associated with your account.
        </p>
      </div>

      {/* GDPR Information */}
      <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-6">
        <h3 className="text-lg font-semibold text-blue-900 dark:text-link-primary mb-2">
          Your GDPR Rights
        </h3>
        <ul className="space-y-2 text-sm text-blue-800 dark:text-blue-300">
          <li className="flex items-start">
            <svg
              className="w-5 h-5 mr-2 mt-0.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span>
              <strong>Right to Access:</strong> You can request a copy of all
              personal data we hold about you.
            </span>
          </li>
          <li className="flex items-start">
            <svg
              className="w-5 h-5 mr-2 mt-0.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span>
              <strong>Right to Rectification:</strong> You can ask us to correct
              inaccurate personal data.
            </span>
          </li>
          <li className="flex items-start">
            <svg
              className="w-5 h-5 mr-2 mt-0.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span>
              <strong>Right to Erasure:</strong> You can request deletion of
              your personal data ("right to be forgotten").
            </span>
          </li>
          <li className="flex items-start">
            <svg
              className="w-5 h-5 mr-2 mt-0.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span>
              <strong>Right to Data Portability:</strong> You can receive your
              data in a machine-readable format.
            </span>
          </li>
          <li className="flex items-start">
            <svg
              className="w-5 h-5 mr-2 mt-0.5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span>
              <strong>Right to Withdraw Consent:</strong> You can withdraw your
              consent at any time.
            </span>
          </li>
        </ul>
      </div>
    </div>
  );

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 dark:bg-gray-900 flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      <div className="max-w-4xl mx-auto px-4 py-8">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
            Privacy Settings
          </h1>
          <p className="text-muted-foreground dark:text-muted-foreground mt-2">
            Manage your data, privacy preferences, and exercise your GDPR rights
          </p>
        </div>

        {/* Error Display */}
        {error && (
          <div className="mb-6 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
            <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
          </div>
        )}

        {/* Tab Navigation */}
        <div className="flex border-b border-gray-200 dark:border-gray-700 mb-6">
          <button
            onClick={() => setActiveTab("overview")}
            className={`px-4 py-2 font-medium text-sm ${
              activeTab === "overview"
                ? "border-b-2 border-blue-600 text-blue-600 dark:text-link-primary"
                : "text-muted-foreground dark:text-muted-foreground hover:text-gray-700 dark:hover:text-gray-200"
            }`}
          >
            Overview
          </button>
          <button
            onClick={() => setActiveTab("consents")}
            className={`px-4 py-2 font-medium text-sm ${
              activeTab === "consents"
                ? "border-b-2 border-blue-600 text-blue-600 dark:text-link-primary"
                : "text-muted-foreground dark:text-muted-foreground hover:text-gray-700 dark:hover:text-gray-200"
            }`}
          >
            Consents
          </button>
          <button
            onClick={() => setActiveTab("export")}
            className={`px-4 py-2 font-medium text-sm ${
              activeTab === "export"
                ? "border-b-2 border-blue-600 text-blue-600 dark:text-link-primary"
                : "text-muted-foreground dark:text-muted-foreground hover:text-gray-700 dark:hover:text-gray-200"
            }`}
          >
            Export Data
          </button>
          <button
            onClick={() => setActiveTab("deletion")}
            className={`px-4 py-2 font-medium text-sm ${
              activeTab === "deletion"
                ? "border-b-2 border-red-600 text-red-600 dark:text-red-400"
                : "text-muted-foreground dark:text-muted-foreground hover:text-gray-700 dark:hover:text-gray-200"
            }`}
          >
            Delete Data
          </button>
        </div>

        {/* Tab Content */}
        {activeTab === "overview" && renderOverview()}
        {activeTab === "consents" && <ConsentManager />}
        {activeTab === "export" && <DataExport />}
        {activeTab === "deletion" && <DataDeletion />}
      </div>
    </div>
  );
}
