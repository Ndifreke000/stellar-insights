"use client";
import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ArrowRight,
  TrendingUp,
  AlertTriangle,
  CheckCircle,
  Clock,
  Zap,
  ArrowRightLeft,
  Info
} from 'lucide-react';
import { getPaymentPrediction, PredictionResponse, AlternativeRoute } from '../../lib/api';

// Common asset options for dropdowns
const ASSETS = ['USDC', 'XLM', 'EURC', 'PHP', 'NGN', 'BRL', 'KES', 'JPY', 'GBP', 'EUR'];

// Risk level colors and icons
const RISK_CONFIG = {
  low: {
    color: 'text-emerald-400',
    bg: 'bg-emerald-500/20',
    border: 'border-emerald-500/30',
    gradient: 'from-emerald-500 to-green-400',
    icon: CheckCircle
  },
  medium: {
    color: 'text-amber-400',
    bg: 'bg-amber-500/20',
    border: 'border-amber-500/30',
    gradient: 'from-amber-500 to-yellow-400',
    icon: AlertTriangle
  },
  high: {
    color: 'text-red-400',
    bg: 'bg-red-500/20',
    border: 'border-red-500/30',
    gradient: 'from-red-500 to-rose-400',
    icon: AlertTriangle
  },
};

// Circular gauge component for success probability
function SuccessGauge({ probability, riskLevel }: { probability: number; riskLevel: 'low' | 'medium' | 'high' }) {
  const percentage = Math.round(probability * 100);
  const circumference = 2 * Math.PI * 45;
  const strokeDashoffset = circumference - (probability * circumference);
  const config = RISK_CONFIG[riskLevel];

  return (
    <div className="relative w-40 h-40 mx-auto">
      <svg className="w-full h-full transform -rotate-90" viewBox="0 0 100 100">
        {/* Background circle */}
        <circle
          cx="50"
          cy="50"
          r="45"
          fill="none"
          stroke="currentColor"
          strokeWidth="8"
          className="text-gray-700/50"
        />
        {/* Progress circle */}
        <motion.circle
          cx="50"
          cy="50"
          r="45"
          fill="none"
          stroke="url(#gaugeGradient)"
          strokeWidth="8"
          strokeLinecap="round"
          strokeDasharray={circumference}
          initial={{ strokeDashoffset: circumference }}
          animate={{ strokeDashoffset }}
          transition={{ duration: 1.5, ease: "easeOut" }}
        />
        <defs>
          <linearGradient id="gaugeGradient" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" className={`stop-color-${riskLevel}`} style={{ stopColor: riskLevel === 'low' ? '#10b981' : riskLevel === 'medium' ? '#f59e0b' : '#ef4444' }} />
            <stop offset="100%" style={{ stopColor: riskLevel === 'low' ? '#34d399' : riskLevel === 'medium' ? '#fbbf24' : '#f87171' }} />
          </linearGradient>
        </defs>
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <motion.span
          className={`text-4xl font-bold ${config.color}`}
          initial={{ opacity: 0, scale: 0.5 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: 0.5, duration: 0.5 }}
        >
          {percentage}%
        </motion.span>
        <span className="text-xs text-muted-foreground uppercase tracking-wider">Success Rate</span>
      </div>
    </div>
  );
}

// Confidence interval bar
function ConfidenceBar({ interval }: { interval: [number, number] }) {
  const [lower, upper] = interval;
  const lowerPct = lower * 100;
  const upperPct = upper * 100;
  const width = upperPct - lowerPct;

  return (
    <div className="space-y-2">
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>0%</span>
        <span className="flex items-center gap-1">
          <Info className="w-3 h-3" />
          Confidence Range
        </span>
        <span>100%</span>
      </div>
      <div className="relative h-3 bg-gray-700/50 rounded-full overflow-hidden">
        <motion.div
          className="absolute h-full bg-gradient-to-r from-blue-500 to-cyan-400 rounded-full"
          initial={{ left: '50%', width: 0 }}
          animate={{ left: `${lowerPct}%`, width: `${width}%` }}
          transition={{ duration: 1, ease: "easeOut", delay: 0.3 }}
        />
        {/* Markers */}
        <motion.div
          className="absolute top-0 w-0.5 h-full bg-white/80"
          initial={{ left: '50%' }}
          animate={{ left: `${lowerPct}%` }}
          transition={{ duration: 1, ease: "easeOut", delay: 0.3 }}
        />
        <motion.div
          className="absolute top-0 w-0.5 h-full bg-white/80"
          initial={{ left: '50%' }}
          animate={{ left: `${upperPct}%` }}
          transition={{ duration: 1, ease: "easeOut", delay: 0.3 }}
        />
      </div>
      <div className="flex justify-center gap-4 text-sm">
        <span className="text-gray-300">
          <span className="text-link-primary font-semibold">{(lower * 100).toFixed(1)}%</span> - <span className="text-cyan-400 font-semibold">{(upper * 100).toFixed(1)}%</span>
        </span>
      </div>
    </div>
  );
}

// Alternative route card
function RouteCard({ route, index }: { route: AlternativeRoute; index: number }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.5 + index * 0.1 }}
      className="p-4 bg-gray-800/50 border border-gray-700/50 rounded-xl hover:border-blue-500/30 transition-all group"
    >
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <div className="p-1.5 bg-blue-500/20 rounded-lg">
            <ArrowRightLeft className="w-4 h-4 text-link-primary" />
          </div>
          <span className="text-sm font-medium text-gray-200">
            {route.source_asset}
            {route.via_asset && <span className="text-muted-foreground"> → {route.via_asset}</span>}
            <span className="text-muted-foreground"> → </span>
            {route.destination_asset}
          </span>
        </div>
        <span className="text-emerald-400 font-semibold text-sm">
          {(route.estimated_success_rate * 100).toFixed(1)}%
        </span>
      </div>
      <p className="text-xs text-muted-foreground">{route.description}</p>
    </motion.div>
  );
}

// Loading skeleton
function LoadingSkeleton() {
  return (
    <div className="space-y-6 animate-pulse">
      <div className="flex justify-center">
        <div className="w-40 h-40 rounded-full bg-gray-700/50" />
      </div>
      <div className="h-3 bg-gray-700/50 rounded-full" />
      <div className="h-20 bg-gray-700/50 rounded-xl" />
      <div className="space-y-3">
        <div className="h-16 bg-gray-700/50 rounded-xl" />
        <div className="h-16 bg-gray-700/50 rounded-xl" />
      </div>
    </div>
  );
}

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
      setError('Failed to get prediction. Please try again.');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-900 to-gray-800">
      {/* Header */}
      <header className="border-b border-gray-800/50">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 py-6">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-gradient-to-br from-blue-500 to-cyan-500 rounded-xl">
              <TrendingUp className="w-6 h-6 text-white" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-white">Corridor Success Rate Predictor</h1>
              <p className="text-sm text-muted-foreground">Predict payment success before you send</p>
            </div>
          </div>
        </div>
      </header>

      <main className="max-w-5xl mx-auto px-4 sm:px-6 py-8">
        <div className="grid lg:grid-cols-2 gap-8">
          {/* Input Form */}
          <div>
            <form onSubmit={handleSubmit} className="p-6 bg-gray-800/30 backdrop-blur-sm border border-gray-700/50 rounded-2xl space-y-6">
              <h2 className="text-lg font-semibold text-white flex items-center gap-2">
                <Zap className="w-5 h-5 text-yellow-400" />
                Payment Details
              </h2>

              {/* Source Asset */}
              <div className="space-y-2">
                <label htmlFor="source-asset" className="block text-sm font-medium text-gray-300">
                  Source Asset
                </label>
                <select
                  id="source-asset"
                  value={sourceAsset}
                  onChange={(e) => setSourceAsset(e.target.value)}
                  className="w-full px-4 py-3 bg-gray-900/50 border border-gray-700 rounded-xl text-white focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-all"
                >
                  {ASSETS.map(asset => (
                    <option key={asset} value={asset}>{asset}</option>
                  ))}
                </select>
              </div>

              {/* Destination Asset */}
              <div className="space-y-2">
                <label htmlFor="dest-asset" className="block text-sm font-medium text-gray-300">
                  Destination Asset
                </label>
                <select
                  id="dest-asset"
                  value={destAsset}
                  onChange={(e) => setDestAsset(e.target.value)}
                  className="w-full px-4 py-3 bg-gray-900/50 border border-gray-700 rounded-xl text-white focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-all"
                >
                  {ASSETS.map(asset => (
                    <option key={asset} value={asset}>{asset}</option>
                  ))}
                </select>
              </div>

              {/* Amount */}
              <div className="space-y-2">
                <label htmlFor="amount" className="block text-sm font-medium text-gray-300">
                  Amount (USD equivalent)
                </label>
                <div className="relative">
                  <span className="absolute left-4 top-1/2 -translate-y-1/2 text-muted-foreground">$</span>
                  <input
                    type="number"
                    id="amount"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    className="w-full pl-8 pr-4 py-3 bg-gray-900/50 border border-gray-700 rounded-xl text-white focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-all"
                    min="0"
                    step="0.01"
                    required
                  />
                </div>
              </div>

              {/* Time of Day */}
              <div className="space-y-2">
                <label htmlFor="time-of-day" className="block text-sm font-medium text-gray-300 flex items-center gap-2">
                  <Clock className="w-4 h-4 text-muted-foreground" />
                  Time of Day (UTC)
                </label>
                <input
                  type="time"
                  id="time-of-day"
                  value={timeOfDay}
                  onChange={(e) => setTimeOfDay(e.target.value)}
                  className="w-full px-4 py-3 bg-gray-900/50 border border-gray-700 rounded-xl text-white focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500 transition-all"
                  required
                />
              </div>

              {/* Submit Button */}
              <motion.button
                type="submit"
                disabled={loading}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                className="w-full py-4 bg-gradient-to-r from-blue-600 to-cyan-600 text-white font-semibold rounded-xl hover:from-blue-500 hover:to-cyan-500 focus:outline-none focus:ring-2 focus:ring-blue-500/50 disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center justify-center gap-2"
              >
                {loading ? (
                  <>
                    <motion.div
                      animate={{ rotate: 360 }}
                      transition={{ duration: 1, repeat: Infinity, ease: "linear" }}
                      className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full"
                    />
                    Analyzing...
                  </>
                ) : (
                  <>
                    Predict Success
                    <ArrowRight className="w-5 h-5" />
                  </>
                )}
              </motion.button>
            </form>
          </div>

          {/* Results Panel */}
          <div className="p-6 bg-gray-800/30 backdrop-blur-sm border border-gray-700/50 rounded-2xl">
            <h2 className="text-lg font-semibold text-white mb-6">Prediction Results</h2>

            <AnimatePresence mode="wait">
              {loading && (
                <motion.div
                  key="loading"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                >
                  <LoadingSkeleton />
                </motion.div>
              )}

              {error && (
                <motion.div
                  key="error"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  className="p-4 bg-red-500/10 border border-red-500/30 rounded-xl text-red-400 text-center"
                >
                  {error}
                </motion.div>
              )}

              {!loading && !error && !prediction && (
                <motion.div
                  key="empty"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  className="flex flex-col items-center justify-center h-64 text-muted-foreground"
                >
                  <TrendingUp className="w-12 h-12 mb-4 opacity-30" />
                  <p className="text-center">Enter payment details and click<br />&quot;Predict Success&quot; to see results</p>
                </motion.div>
              )}

              {prediction && !loading && (
                <motion.div
                  key="results"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  className="space-y-6"
                >
                  {/* Success Gauge */}
                  <SuccessGauge
                    probability={prediction.success_probability}
                    riskLevel={prediction.risk_level}
                  />

                  {/* Risk Badge */}
                  <motion.div
                    initial={{ opacity: 0, scale: 0.9 }}
                    animate={{ opacity: 1, scale: 1 }}
                    transition={{ delay: 0.3 }}
                    className={`p-4 rounded-xl ${RISK_CONFIG[prediction.risk_level].bg} ${RISK_CONFIG[prediction.risk_level].border} border`}
                  >
                    <div className="flex items-start gap-3">
                      {React.createElement(RISK_CONFIG[prediction.risk_level].icon, {
                        className: `w-5 h-5 ${RISK_CONFIG[prediction.risk_level].color} shrink-0 mt-0.5`
                      })}
                      <div>
                        <div className={`text-sm font-semibold ${RISK_CONFIG[prediction.risk_level].color} uppercase tracking-wide mb-1`}>
                          {prediction.risk_level} Risk
                        </div>
                        <p className="text-sm text-gray-300">{prediction.recommendation}</p>
                      </div>
                    </div>
                  </motion.div>

                  {/* Confidence Interval */}
                  <ConfidenceBar interval={prediction.confidence_interval} />

                  {/* Alternative Routes */}
                  {prediction.alternative_routes.length > 0 && (
                    <div className="space-y-3">
                      <h3 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                        Better Routes Available
                      </h3>
                      {prediction.alternative_routes.map((route, index) => (
                        <RouteCard key={index} route={route} index={index} />
                      ))}
                    </div>
                  )}

                  {/* Model Info */}
                  <div className="pt-4 border-t border-gray-700/50 text-xs text-muted-foreground text-center">
                    Model version: {prediction.model_version}
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </div>
      </main>
    </div>
  );
};

export default PredictionForm;
