"use client";
import React, { useEffect, useState } from 'react';
import { getAnchors, AnchorMetrics } from '../../lib/api';
import StatusBadge from './status-badge';

const HealthDashboard = () => {
  const [anchors, setAnchors] = useState<AnchorMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchAnchors = async () => {
      try {
        const response = await getAnchors();
        setAnchors(response.anchors);
      } catch (err) {
        setError('Failed to fetch anchor data.');
        console.error(err);
      } finally {
        setLoading(false);
      }
    };

    fetchAnchors();
  }, []);

  if (loading) {
    return <div className="text-center p-8">Loading anchor health data...</div>;
  }

  if (error) {
    return <div className="text-center p-8 text-red-500">{error}</div>;
  }

  return (
    <div className="container mx-auto p-4">
      <header className="mb-8">
        <h1 className="text-3xl font-bold text-gray-800">Anchor Health Monitoring</h1>
        <p className="text-gray-600">Real-time status and performance of network anchors.</p>
      </header>
      
      <div className="grid grid-cols-1 md:grid-cols-1 lg:grid-cols-2 xl:grid-cols-2 gap-6">
        {anchors.map((anchor) => (
          <div key={anchor.id} className="bg-white p-6 border rounded-lg shadow-md hover:shadow-lg transition-shadow">
            <div className="flex justify-between items-start mb-4">
              <h2 className="font-bold text-xl text-gray-900">{anchor.name}</h2>
              <StatusBadge status={anchor.status} />
            </div>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="p-4 bg-gray-50 rounded-lg">
                <h3 className="font-semibold text-gray-700 mb-2">Health Score</h3>
                <p className="text-4xl font-bold text-blue-600">{anchor.reliability_score.toFixed(1)}%</p>
              </div>
              <div className="p-4 bg-gray-50 rounded-lg">
                <h3 className="font-semibold text-gray-700 mb-2">Key Metrics</h3>
                <p className="text-sm">Uptime: <span className="font-semibold">
                  {anchor.total_transactions > 0 
                    ? ((anchor.successful_transactions / anchor.total_transactions) * 100).toFixed(2)
                    : 'N/A'}%
                </span></p>
                 <p className="text-sm">Failure Rate: <span className="font-semibold">{anchor.failure_rate.toFixed(2)}%</span></p>
                 <p className="text-sm">Total Transactions: <span className="font-semibold">{anchor.total_transactions}</span></p>
              </div>
            </div>

            <div className="mt-6">
              <h3 className="font-semibold text-gray-700">Recent Failures</h3>
              <div className="p-4 mt-2 bg-gray-100 rounded-lg text-sm text-gray-500">
                Coming soon: A list of recent transaction failures.
              </div>
            </div>

            <div className="mt-6">
              <h3 className="font-semibold text-gray-700">Historical Health</h3>
              <div className="p-4 mt-2 bg-gray-100 rounded-lg text-sm text-gray-500">
                Coming soon: A chart showing health score over time.
              </div>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-10 p-6 bg-white border rounded-lg shadow-md">
        <h2 className="font-bold text-xl text-gray-900 mb-4">Alert Threshold Configuration</h2>
        <div className="p-4 bg-gray-100 rounded-lg text-sm text-gray-500">
          Coming soon: Configure email or webhook alerts for when an anchor's health score drops below a certain threshold.
        </div>
      </div>
    </div>
  );
};

export default HealthDashboard;
