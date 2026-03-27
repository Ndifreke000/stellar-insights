"use client";

import React, { useState, useEffect } from "react";
import {
  getDeletionRequests,
  createDeletionRequest,
  cancelDeletionRequest,
  DeletionRequestResponse,
} from "@/lib/gdpr-api";

interface DataDeletionProps {
  onClose?: () => void;
}

export function DataDeletion({ onClose }: DataDeletionProps) {
  const [deletionRequests, setDeletionRequests] = useState<
    DeletionRequestResponse[]
  >([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [showConfirmation, setShowConfirmation] = useState(false);
  const [deletionReason, setDeletionReason] = useState("");
  const [deleteAllData, setDeleteAllData] = useState(true);
  const [activeTab, setActiveTab] = useState<"new" | "history">("new");

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);
      const requests = await getDeletionRequests();
      setDeletionRequests(requests);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to load deletion requests",
      );
    } finally {
      setLoading(false);
    }
  };

  const handleCreateDeletion = async () => {
    try {
      setCreating(true);
      setError(null);
      setSuccess(null);

      const result = await createDeletionRequest({
        reason: deletionReason || undefined,
        delete_all_data: deleteAllData,
      });

      if (result.confirmation_required && result.confirmation_token) {
        setSuccess(
          `Deletion request created! Please save this confirmation token: ${result.confirmation_token}. You will need to confirm the deletion within 24 hours.`,
        );
      } else {
        setSuccess("Deletion request created successfully.");
      }

      setDeletionReason("");
      setDeleteAllData(true);
      setShowConfirmation(false);
      await loadData();
      setActiveTab("history");
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : "Failed to create deletion request",
      );
    } finally {
      setCreating(false);
    }
  };

  const handleCancelRequest = async (id: string) => {
    if (!confirm("Are you sure you want to cancel this deletion request?")) {
      return;
    }

    try {
      setError(null);
      await cancelDeletionRequest(id);
      setSuccess("Deletion request cancelled successfully.");
      await loadData();
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : "Failed to cancel deletion request",
      );
    }
  };

  const getStatusBadge = (status: string) => {
    const statusStyles: Record<string, string> = {
      pending:
        "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400",
      scheduled:
        "bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-400",
      processing:
        "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-link-primary",
      completed:
        "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400",
      cancelled:
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
          Delete Your Data
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

      {/* Warning Banner */}
      <div className="mb-6 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
        <div className="flex items-start">
          <svg
            className="w-5 h-5 text-red-600 dark:text-red-400 mt-0.5 mr-3 flex-shrink-0"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          <div>
            <h3 className="text-sm font-medium text-red-800 dark:text-red-400">
              Important: Data Deletion is Permanent
            </h3>
            <p className="text-sm text-red-700 dark:text-red-300 mt-1">
              Once your data is deleted, it cannot be recovered. Please make
              sure to export any important data before requesting deletion.
            </p>
          </div>
        </div>
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
          Request Deletion
        </button>
        <button
          onClick={() => setActiveTab("history")}
          className={`px-4 py-2 font-medium text-sm ${
            activeTab === "history"
              ? "border-b-2 border-blue-600 text-blue-600 dark:text-link-primary"
              : "text-muted-foreground dark:text-muted-foreground hover:text-gray-700 dark:hover:text-gray-200"
          }`}
        >
          Request History
        </button>
      </div>

      {error && (
        <div className="mb-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
          <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
        </div>
      )}

      {success && (
        <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-md">
          <p className="text-sm text-green-600 dark:text-green-400 whitespace-pre-wrap">
            {success}
          </p>
        </div>
      )}

      {activeTab === "new" && !showConfirmation && (
        <div>
          <p className="text-muted-foreground dark:text-muted-foreground mb-6">
            You can request to delete all your personal data, or select specific
            data types to delete. This action cannot be undone.
          </p>

          <div className="space-y-4 mb-6">
            <label className="flex items-start p-4 border border-gray-200 dark:border-gray-700 rounded-lg cursor-pointer hover:border-gray-300 dark:hover:border-gray-600">
              <input
                type="radio"
                name="deleteScope"
                checked={deleteAllData}
                onChange={() => setDeleteAllData(true)}
                className="mt-1 mr-3"
              />
              <div>
                <p className="font-medium text-gray-900 dark:text-white">
                  Delete All My Data
                </p>
                <p className="text-sm text-muted-foreground dark:text-muted-foreground mt-1">
                  Permanently delete all personal data associated with your
                  account
                </p>
              </div>
            </label>

            <label className="flex items-start p-4 border border-gray-200 dark:border-gray-700 rounded-lg cursor-pointer hover:border-gray-300 dark:hover:border-gray-600">
              <input
                type="radio"
                name="deleteScope"
                checked={!deleteAllData}
                onChange={() => setDeleteAllData(false)}
                className="mt-1 mr-3"
              />
              <div>
                <p className="font-medium text-gray-900 dark:text-white">
                  Delete Specific Data
                </p>
                <p className="text-sm text-muted-foreground dark:text-muted-foreground mt-1">
                  Choose which specific data types to delete (coming soon)
                </p>
              </div>
            </label>
          </div>

          <div className="mb-6">
            <label
              htmlFor="reason"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
            >
              Reason for deletion (optional)
            </label>
            <textarea
              id="reason"
              value={deletionReason}
              onChange={(e) => setDeletionReason(e.target.value)}
              rows={3}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              placeholder="Please tell us why you're requesting deletion..."
            />
          </div>

          <button
            onClick={() => setShowConfirmation(true)}
            className="w-full px-6 py-3 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
          >
            Request Data Deletion
          </button>
        </div>
      )}

      {activeTab === "new" && showConfirmation && (
        <div className="text-center">
          <div className="mb-6">
            <div className="mx-auto w-16 h-16 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center">
              <svg
                className="w-8 h-8 text-red-600 dark:text-red-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                />
              </svg>
            </div>
          </div>

          <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
            Confirm Data Deletion
          </h3>
          <p className="text-muted-foreground dark:text-muted-foreground mb-6">
            Are you absolutely sure you want to delete your data? This action is
            irreversible.
          </p>

          <div className="flex gap-3 justify-center">
            <button
              onClick={() => setShowConfirmation(false)}
              className="px-6 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleCreateDeletion}
              disabled={creating}
              className="px-6 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50 transition-colors"
            >
              {creating ? "Processing..." : "Yes, Delete My Data"}
            </button>
          </div>
        </div>
      )}

      {activeTab === "history" && (
        <div>
          {deletionRequests.length === 0 ? (
            <p className="text-center text-muted-foreground dark:text-muted-foreground py-8">
              No deletion requests yet.
            </p>
          ) : (
            <div className="space-y-3">
              {deletionRequests.map((request) => (
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
                    {request.scheduled_deletion_at && (
                      <p className="text-xs text-orange-600 dark:text-orange-400">
                        Scheduled:{" "}
                        {new Date(
                          request.scheduled_deletion_at,
                        ).toLocaleString()}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-3">
                    {getStatusBadge(request.status)}
                    {(request.status === "pending" ||
                      request.status === "scheduled") && (
                      <button
                        onClick={() => handleCancelRequest(request.id)}
                        className="px-3 py-1 text-sm text-muted-foreground dark:text-muted-foreground hover:text-gray-800 dark:hover:text-gray-200 transition-colors"
                      >
                        Cancel
                      </button>
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

export default DataDeletion;
