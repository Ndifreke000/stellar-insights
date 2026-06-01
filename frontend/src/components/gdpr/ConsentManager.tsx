"use client";

import React, { useState, useEffect } from "react";
import {
  getConsents,
  batchUpdateConsents,
  getGdprSummary,
  CONSENT_LABELS,
  ConsentResponse,
  GdprSummary,
} from "@/lib/gdpr-api";

interface ConsentManagerProps {
  onClose?: () => void;
}

export function ConsentManager({ onClose }: ConsentManagerProps) {
  const [consents, setConsents] = useState<ConsentResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    loadConsents();
  }, []);

  const loadConsents = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await getConsents();
      setConsents(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load consents");
    } finally {
      setLoading(false);
    }
  };

  const handleConsentChange = (consentType: string, consentGiven: boolean) => {
    setConsents((prev) =>
      prev.map((c) =>
        c.consent_type === consentType
          ? { ...c, consent_given: consentGiven }
          : c,
      ),
    );
  };

  const handleSaveConsents = async () => {
    try {
      setSaving(true);
      setError(null);
      setSuccess(null);

      const consentsToUpdate = consents.map((c) => ({
        consent_type: c.consent_type,
        consent_given: c.consent_given,
      }));

      await batchUpdateConsents(consentsToUpdate);
      setSuccess("Your preferences have been saved successfully.");
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to save preferences",
      );
    } finally {
      setSaving(false);
    }
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
          Privacy Preferences
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

      <div className="space-y-4">
        {consents.map((consent) => {
          const labels = CONSENT_LABELS[consent.consent_type] || {
            title: consent.consent_type,
            description: "",
          };

          return (
            <div
              key={consent.consent_type}
              className="flex items-start justify-between p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg"
            >
              <div className="flex-1 pr-4">
                <h3 className="font-medium text-gray-900 dark:text-white">
                  {labels.title}
                </h3>
                <p className="text-sm text-muted-foreground dark:text-muted-foreground mt-1">
                  {labels.description}
                </p>
                {consent.granted_at && (
                  <p className="text-xs text-muted-foreground dark:text-muted-foreground mt-2">
                    Granted on:{" "}
                    {new Date(consent.granted_at).toLocaleDateString()}
                  </p>
                )}
              </div>
              <label className="relative inline-flex items-center cursor-pointer">
                <input
                  type="checkbox"
                  checked={consent.consent_given}
                  onChange={(e) =>
                    handleConsentChange(consent.consent_type, e.target.checked)
                  }
                  className="sr-only peer"
                />
                <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-600 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-500 peer-checked:bg-blue-600"></div>
              </label>
            </div>
          );
        })}
      </div>

      <div className="mt-6 flex justify-end">
        <button
          onClick={handleSaveConsents}
          disabled={saving}
          className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {saving ? "Saving..." : "Save Preferences"}
        </button>
      </div>
    </div>
  );
}

export default ConsentManager;
