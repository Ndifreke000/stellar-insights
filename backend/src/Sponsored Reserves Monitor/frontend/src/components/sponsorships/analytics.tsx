import React from "react";

interface SponsorshipAnalyticsType {
  total_sponsorships: number;
  total_amount_sponsored: string;
  unique_sponsors: number;
  unique_sponsored_accounts: number;
  average_sponsorship: string;
  largest_sponsorship: string;
  smallest_sponsorship: string;
}

interface Props {
  analytics: SponsorshipAnalyticsType;
}

export default function SponsorshipAnalytics({ analytics }: Props) {
  const stats = [
    {
      label: "Total Sponsorships",
      value: analytics.total_sponsorships,
      icon: "📊",
      color: "bg-blue-50 text-blue-700",
    },
    {
      label: "Unique Sponsors",
      value: analytics.unique_sponsors,
      icon: "👥",
      color: "bg-green-50 text-green-700",
    },
    {
      label: "Sponsored Accounts",
      value: analytics.unique_sponsored_accounts,
      icon: "🔑",
      color: "bg-purple-50 text-purple-700",
    },
    {
      label: "Total Amount",
      value: analytics.total_amount_sponsored,
      icon: "💰",
      color: "bg-yellow-50 text-yellow-700",
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      {stats.map((stat) => (
        <div
          key={stat.label}
          className={`${stat.color} rounded-lg p-6 shadow-sm border border-current border-opacity-10`}
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">{stat.label}</p>
              <p className="text-2xl font-bold mt-2">{stat.value}</p>
            </div>
            <span className="text-3xl">{stat.icon}</span>
          </div>
        </div>
      ))}
    </div>
  );
}
