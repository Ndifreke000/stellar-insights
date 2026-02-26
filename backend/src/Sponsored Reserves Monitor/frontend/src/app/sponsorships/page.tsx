"use client";

import React, { useEffect, useState } from "react";
import axios from "axios";
import SponsorshipLeaderboard from "@/components/sponsorships/leaderboard";
import SponsorshipAnalytics from "@/components/sponsorships/analytics";
import SponsorshipList from "@/components/sponsorships/list";

export default function SponsorshipsPage() {
  const [sponsorships, setSponsorships] = useState([]);
  const [leaderboard, setLeaderboard] = useState([]);
  const [analytics, setAnalytics] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [activeTab, setActiveTab] = useState("overview");

  useEffect(() => {
    fetchData();
  }, []);

  const fetchData = async () => {
    try {
      setLoading(true);
      const [sponsorshipsRes, leaderboardRes, analyticsRes] = await Promise.all(
        [
          axios.get("/api/sponsorships?limit=100"),
          axios.get("/api/sponsors/leaderboard?limit=50"),
          axios.get("/api/analytics/summary"),
        ],
      );

      setSponsorships(sponsorshipsRes.data);
      setLeaderboard(leaderboardRes.data);
      setAnalytics(analyticsRes.data);
    } catch (err) {
      setError(err.message || "Failed to fetch data");
      console.error("Error fetching sponsorship data:", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900">
            Stellar Sponsored Reserves Monitor
          </h1>
          <p className="mt-2 text-gray-600">
            Track and analyze sponsorship relationships on Stellar
          </p>
        </div>

        {/* Refresh Button */}
        <div className="mb-6">
          <button
            onClick={fetchData}
            disabled={loading}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-400"
          >
            {loading ? "Refreshing..." : "Refresh Data"}
          </button>
        </div>

        {/* Error Message */}
        {error && (
          <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700">
            Error: {error}
          </div>
        )}

        {/* Analytics */}
        {analytics && <SponsorshipAnalytics analytics={analytics} />}

        {/* Tabs */}
        <div className="mt-8 border-b border-gray-200">
          <nav className="flex space-x-8">
            <button
              onClick={() => setActiveTab("overview")}
              className={`py-4 px-1 border-b-2 font-medium text-sm ${
                activeTab === "overview"
                  ? "border-blue-500 text-blue-600"
                  : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
              }`}
            >
              Leaderboard
            </button>
            <button
              onClick={() => setActiveTab("list")}
              className={`py-4 px-1 border-b-2 font-medium text-sm ${
                activeTab === "list"
                  ? "border-blue-500 text-blue-600"
                  : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
              }`}
            >
              All Sponsorships
            </button>
          </nav>
        </div>

        {/* Tab Content */}
        <div className="mt-8">
          {activeTab === "overview" && (
            <SponsorshipLeaderboard data={leaderboard} />
          )}
          {activeTab === "list" && (
            <SponsorshipList sponsorships={sponsorships} />
          )}
        </div>
      </div>
    </div>
  );
}
