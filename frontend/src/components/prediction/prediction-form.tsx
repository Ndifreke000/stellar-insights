"use client";
import React, { useState } from 'react';
import { getPaymentPrediction, PredictionResponse } from '../../lib/api';

const PredictionForm = () => {
  const [sourceAsset, setSourceAsset] = useState('USDC');
  const [destAsset, setDestAsset] = useState('XLM');
  const [amount, setAmount] = useState('100.0');
  const [timeOfDay, setTimeOfDay] = useState('12:00');
  
  const [prediction, setPrediction] = useState<PredictionResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setPrediction(null);

    try {
      const response = await getPaymentPrediction({
        source_asset: sourceAsset,
        destination_asset: destAsset,
        amount: parseFloat(amount),
        time_of_day: timeOfDay,
      });
      setPrediction(response);
    } catch (err) {
      setError('Failed to get prediction.');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="container mx-auto p-8 bg-gray-50 min-h-screen">
      <header className="mb-8">
        <h1 className="text-3xl font-bold text-gray-800">Payment Success Prediction</h1>
        <p className="text-gray-600">Predict the likelihood of a payment succeeding based on corridor, amount, and time.</p>
      </header>
      
      <form onSubmit={handleSubmit} className="p-6 bg-white border rounded-lg shadow-md">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <div>
            <label htmlFor="source-asset" className="block text-sm font-medium text-gray-700">Source Asset</label>
            <input
              type="text"
              id="source-asset"
              value={sourceAsset}
              onChange={(e) => setSourceAsset(e.target.value)}
              className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
              required
            />
          </div>
          <div>
            <label htmlFor="dest-asset" className="block text-sm font-medium text-gray-700">Destination Asset</label>
            <input
              type="text"
              id="dest-asset"
              value={destAsset}
              onChange={(e) => setDestAsset(e.target.value)}
              className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
              required
            />
          </div>
          <div>
            <label htmlFor="amount" className="block text-sm font-medium text-gray-700">Amount</label>
            <input
              type="number"
              id="amount"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
              required
            />
          </div>
          <div>
            <label htmlFor="time-of-day" className="block text-sm font-medium text-gray-700">Time of Day</label>
            <input
              type="time"
              id="time-of-day"
              value={timeOfDay}
              onChange={(e) => setTimeOfDay(e.target.value)}
              className="mt-1 block w-full px-3 py-2 bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
              required
            />
          </div>
        </div>
        <div className="mt-6 text-right">
          <button type="submit" className="px-6 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500" disabled={loading}>
            {loading ? 'Predicting...' : 'Predict Success'}
          </button>
        </div>
      </form>

      {error && <div className="mt-6 text-red-500 text-center">{error}</div>}

      {prediction && (
        <div className="mt-8">
          <div className="bg-white p-6 border rounded-lg shadow-md">
            <h2 className="text-xl font-bold mb-4">Prediction Results</h2>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <div className="text-center">
                <p className="text-lg text-gray-600">Success Probability</p>
                <p className="text-5xl font-bold text-green-600">{(prediction.success_probability * 100).toFixed(2)}%</p>
              </div>
              <div className="text-center">
                <p className="text-lg text-gray-600">Confidence Interval</p>
                <p className="text-3xl font-semibold text-gray-800">
                  ({(prediction.confidence_interval[0] * 100).toFixed(2)}% - {(prediction.confidence_interval[1] * 100).toFixed(2)}%)
                </p>
              </div>
            </div>
          </div>

          <div className="mt-6 bg-white p-6 border rounded-lg shadow-md">
            <h2 className="text-xl font-bold mb-4">Alternative Route Suggestions</h2>
            <ul className="list-disc list-inside space-y-2">
              {prediction.alternative_routes.map((route, index) => (
                <li key={index} className="text-gray-700">{route}</li>
              ))}
            </ul>
          </div>
        </div>
      )}
    </div>
  );
};

export default PredictionForm;

