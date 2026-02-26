import React from "react";

interface Sponsorship {
  id: string;
  sponsor: string;
  sponsored_account: string;
  sponsored_reserves: number;
  total_amount: string;
  created_at: string;
  updated_at: string;
}

interface Props {
  sponsorships: Sponsorship[];
}

export default function SponsorshipList({ sponsorships }: Props) {
  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString();
  };

  if (sponsorships.length === 0) {
    return (
      <div className="text-center py-12">
        <p className="text-gray-500">No sponsorships found</p>
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead className="bg-gray-100">
          <tr>
            <th className="px-6 py-3 text-left font-semibold text-gray-900">
              Sponsor
            </th>
            <th className="px-6 py-3 text-left font-semibold text-gray-900">
              Sponsored Account
            </th>
            <th className="px-6 py-3 text-left font-semibold text-gray-900">
              Reserves
            </th>
            <th className="px-6 py-3 text-left font-semibold text-gray-900">
              Amount
            </th>
            <th className="px-6 py-3 text-left font-semibold text-gray-900">
              Created
            </th>
          </tr>
        </thead>
        <tbody className="bg-white divide-y divide-gray-200">
          {sponsorships.map((s) => (
            <tr key={s.id} className="hover:bg-gray-50">
              <td className="px-6 py-4 font-mono text-xs text-gray-600 truncate">
                {s.sponsor}
              </td>
              <td className="px-6 py-4 font-mono text-xs text-gray-600 truncate">
                {s.sponsored_account}
              </td>
              <td className="px-6 py-4 text-gray-900">
                {s.sponsored_reserves}
              </td>
              <td className="px-6 py-4 font-semibold text-gray-900">
                {s.total_amount}
              </td>
              <td className="px-6 py-4 text-gray-600">
                {formatDate(s.created_at)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
