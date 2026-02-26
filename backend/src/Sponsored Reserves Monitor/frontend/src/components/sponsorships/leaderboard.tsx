import React from "react";

interface SponsorLeaderboard {
  sponsor: string;
  total_sponsored_amount: string;
  sponsored_accounts_count: number;
  rank: number;
}

interface Props {
  data: SponsorLeaderboard[];
}

export default function SponsorshipLeaderboard({ data }: Props) {
  if (data.length === 0) {
    return (
      <div className="text-center py-12">
        <p className="text-gray-500">No sponsor data yet</p>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      <div className="p-6 border-b border-gray-200">
        <h2 className="text-xl font-bold text-gray-900">
          Top Sponsors Leaderboard
        </h2>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="bg-gray-100">
            <tr>
              <th className="px-6 py-3 text-left font-semibold text-gray-900">
                Rank
              </th>
              <th className="px-6 py-3 text-left font-semibold text-gray-900">
                Sponsor
              </th>
              <th className="px-6 py-3 text-left font-semibold text-gray-900">
                Total Sponsored
              </th>
              <th className="px-6 py-3 text-left font-semibold text-gray-900">
                Accounts Sponsored
              </th>
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-gray-200">
            {data.map((sponsor) => (
              <tr key={sponsor.sponsor} className="hover:bg-gray-50">
                <td className="px-6 py-4 text-gray-900 font-semibold">
                  #{sponsor.rank}
                </td>
                <td className="px-6 py-4 font-mono text-xs text-gray-600 truncate">
                  {sponsor.sponsor}
                </td>
                <td className="px-6 py-4 font-semibold text-gray-900">
                  {sponsor.total_sponsored_amount}
                </td>
                <td className="px-6 py-4 text-gray-900 text-center">
                  {sponsor.sponsored_accounts_count}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
