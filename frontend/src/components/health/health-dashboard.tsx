"use client";
import React, { useEffect, useState } from 'react';
import { ResponsiveContainer, LineChart, Line, XAxis, YAxis, Tooltip, AreaChart, Area } from 'recharts';
import { AlertTriangle, Settings, Activity, ShieldCheck, Zap, Server } from 'lucide-react';
import { getAnchors, AnchorMetrics } from '../../lib/api';
import { Badge } from '@/components/ui/badge';
import { MetricCard } from '@/components/dashboard/MetricCard';

interface AlertThreshold {
  healthScore: number;
  uptimePercentage: number;
  enabled: boolean;
}

interface FailureRecord {
  timestamp: string;
  reason: string;
  corridor?: string;
}

const generateHistoricalData = (currentScore: number) => {
  const data = [];
  for (let i = 29; i >= 0; i--) {
    const date = new Date(Date.now() - i * 24 * 60 * 60 * 1000);
    const variation = (Math.random() - 0.5) * 10;
    const score = Math.max(0, Math.min(100, currentScore + variation));
    data.push({
      date: date.toISOString().split('T')[0],
      score: Math.round(score * 10) / 10
    });
  }
  return data;
};

const generateRecentFailures = (): FailureRecord[] => {
  const reasons = ['Timeout', 'Insufficient liquidity', 'Path payment failed', 'Network congestion'];
  const corridors = ['USDC-PHP', 'EURC-NGN', 'USDT-KES', 'XLM-USD'];

  return Array.from({ length: Math.floor(Math.random() * 5) + 1 }, (_, i) => ({
    timestamp: new Date(Date.now() - Math.random() * 24 * 60 * 60 * 1000).toISOString(),
    reason: reasons[Math.floor(Math.random() * reasons.length)],
    corridor: corridors[Math.floor(Math.random() * corridors.length)]
  })).sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
};

const HealthDashboard = () => {
  const [anchors, setAnchors] = useState<AnchorMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [alertThresholds, setAlertThresholds] = useState<AlertThreshold>({
    healthScore: 85,
    uptimePercentage: 95,
    enabled: true
  });
  const [showSettings, setShowSettings] = useState(false);

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

  const calculateUptime = (anchor: AnchorMetrics): number => {
    return anchor.total_transactions > 0
      ? (anchor.successful_transactions / anchor.total_transactions) * 100
      : 0;
  };

  if (loading) {
    return (
      <div className="flex h-[80vh] items-center justify-center">
        <div className="text-sm font-mono text-accent animate-pulse uppercase tracking-widest italic">Scanning Network Pulse... // 909-Y</div>
      </div>
    );
  }

  const healthyAnchors = anchors.filter(a => a.reliability_score >= 95).length;
  const warningAnchors = anchors.filter(a => a.reliability_score >= 85 && a.reliability_score < 95).length;
  const criticalAnchors = anchors.filter(a => a.reliability_score < 85).length;

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      {/* Page Header */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">Systems Status // 04</div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <ShieldCheck className="w-8 h-8 text-accent" />
            Network Health
          </h2>
        </div>
        <button
          onClick={() => setShowSettings(!showSettings)}
          className={`px-4 py-2 border rounded-lg text-[10px] font-bold uppercase tracking-widest transition-all ${showSettings ? 'bg-accent text-white border-accent glow-accent' : 'glass text-muted-foreground hover:border-accent/50'
            }`}
        >
          Alert Config
        </button>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <MetricCard label="Total Nodes" value={anchors.length} subLabel="Active Anchors" />
        <MetricCard label="Stable" value={healthyAnchors} subLabel=">= 95% Health" trend={100} trendDirection="up" />
        <MetricCard label="Degraded" value={warningAnchors} subLabel="< 95% Health" />
        <MetricCard label="Critical" value={criticalAnchors} subLabel="Action Required" />
      </div>

      {/* Settings Panel */}
      {showSettings && (
        <div className="glass-card rounded-2xl p-6 animate-in zoom-in-95 duration-300">
          <div className="flex items-center gap-2 mb-6 text-accent">
            <Settings className="w-4 h-4" />
            <h3 className="text-xs font-mono uppercase tracking-[0.2em]">Alert Thresholds</h3>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="space-y-2">
              <label className="text-[10px] font-mono text-muted-foreground uppercase">Health Score (%)</label>
              <input
                type="number"
                value={alertThresholds.healthScore}
                onChange={(e) => setAlertThresholds(prev => ({ ...prev, healthScore: Number(e.target.value) }))}
                className="w-full bg-slate-950/50 border border-border/50 rounded-lg px-3 py-2 text-xs font-mono focus:outline-none focus:ring-1 focus:ring-accent"
              />
            </div>
            <div className="space-y-2">
              <label className="text-[10px] font-mono text-muted-foreground uppercase">Uptime (%)</label>
              <input
                type="number"
                value={alertThresholds.uptimePercentage}
                onChange={(e) => setAlertThresholds(prev => ({ ...prev, uptimePercentage: Number(e.target.value) }))}
                className="w-full bg-slate-950/50 border border-border/50 rounded-lg px-3 py-2 text-xs font-mono focus:outline-none focus:ring-1 focus:ring-accent"
              />
            </div>
            <div className="flex items-end pb-2">
              <label className="flex items-center gap-3 cursor-pointer group">
                <input
                  type="checkbox"
                  checked={alertThresholds.enabled}
                  onChange={(e) => setAlertThresholds(prev => ({ ...prev, enabled: e.target.checked }))}
                  className="w-4 h-4 rounded border-border/50 bg-slate-950/50 text-accent focus:ring-accent/50"
                />
                <span className="text-[10px] font-mono text-muted-foreground uppercase group-hover:text-accent transition-colors">Broadcast Alerts</span>
              </label>
            </div>
          </div>
        </div>
      )}

      {/* Anchor Health Cards */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {anchors.map((anchor) => {
          const uptime = calculateUptime(anchor);
          const historicalData = generateHistoricalData(anchor.reliability_score);
          const recentFailures = generateRecentFailures();
          const score = anchor.reliability_score;

          return (
            <div key={anchor.id} className="glass-card rounded-2xl p-6 border border-border/50 hover:border-accent/20 transition-all duration-300">
              <div className="flex justify-between items-start mb-6">
                <div>
                  <h2 className="text-xl font-bold tracking-tight text-foreground">{anchor.name}</h2>
                  <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mt-1">
                    {anchor.stellar_account.slice(0, 12)}...{anchor.stellar_account.slice(-12)}
                  </p>
                </div>
                <Badge variant="outline" className={`font-mono text-[10px] uppercase border-none px-3 py-1 ${score >= 95 ? 'bg-green-500/10 text-green-400' :
                    score >= 85 ? 'bg-yellow-500/10 text-yellow-400' : 'bg-red-500/10 text-red-400'
                  }`}>
                  {score >= 95 ? 'Optimal' : score >= 85 ? 'Degraded' : 'Critical'}
                </Badge>
              </div>

              <div className="grid grid-cols-2 gap-4 mb-8">
                <div className="p-4 rounded-xl bg-slate-900/30 border border-white/5">
                  <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider mb-2">Stability Index</div>
                  <div className="flex items-center gap-3">
                    <span className="text-3xl font-black font-mono tracking-tighter">
                      {score.toFixed(1)}%
                    </span>
                    <div className={`w-2 h-2 rounded-full animate-pulse ${score >= 95 ? 'bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.5)]' :
                        score >= 85 ? 'bg-yellow-500 shadow-[0_0_8px_rgba(234,179,8,0.5)]' : 'bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.5)]'
                      }`} />
                  </div>
                </div>
                <div className="p-4 rounded-xl bg-slate-900/30 border border-white/5">
                  <div className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider mb-2">Tx Uptime</div>
                  <div className="text-3xl font-black font-mono tracking-tighter">
                    {uptime.toFixed(1)}%
                  </div>
                  <div className="text-[9px] font-mono text-muted-foreground/50 mt-1 uppercase">
                    {anchor.successful_transactions.toLocaleString()} / {anchor.total_transactions.toLocaleString()} TX_PASS
                  </div>
                </div>
              </div>

              {/* Historical Health Chart */}
              <div className="mb-8">
                <div className="flex items-center gap-2 mb-4">
                  <Activity className="w-3 h-3 text-accent" />
                  <h3 className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest">30-Day Reliability Telemetry</h3>
                </div>
                <div className="h-24 opacity-80">
                  <ResponsiveContainer width="100%" height="100%">
                    <AreaChart data={historicalData}>
                      <defs>
                        <linearGradient id="colorScore" x1="0" y1="0" x2="0" y2="1">
                          <stop offset="5%" stopColor="#6366f1" stopOpacity={0.3} />
                          <stop offset="95%" stopColor="#6366f1" stopOpacity={0} />
                        </linearGradient>
                      </defs>
                      <Tooltip
                        contentStyle={{ backgroundColor: '#0f172a', border: '1px solid rgba(255,255,255,0.1)', fontSize: '10px', fontFamily: 'monospace' }}
                        itemStyle={{ color: '#6366f1' }}
                      />
                      <Area
                        type="monotone"
                        dataKey="score"
                        stroke="#6366f1"
                        fillOpacity={1}
                        fill="url(#colorScore)"
                        strokeWidth={2}
                      />
                    </AreaChart>
                  </ResponsiveContainer>
                </div>
              </div>

              {/* Recent Failures */}
              <div>
                <div className="flex items-center gap-2 mb-4">
                  <AlertTriangle className="w-3 h-3 text-red-500" />
                  <h3 className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest">Incident Log</h3>
                </div>
                <div className="space-y-2 max-h-32 overflow-y-auto pr-2 custom-scrollbar">
                  {recentFailures.length > 0 ? (
                    recentFailures.map((failure, index) => (
                      <div key={index} className="flex justify-between items-center p-3 glass rounded-lg border-l-2 border-red-500/50">
                        <div className="flex items-center gap-3">
                          <span className="text-[10px] font-mono text-red-400 font-bold">{failure.reason}</span>
                          {failure.corridor && (
                            <Badge variant="outline" className="text-[8px] font-mono border-red-500/20 text-red-400/70">{failure.corridor}</Badge>
                          )}
                        </div>
                        <span className="text-[9px] font-mono text-muted-foreground">
                          {new Date(failure.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                        </span>
                      </div>
                    ))
                  ) : (
                    <div className="p-4 glass rounded-xl border-dashed flex items-center justify-center">
                      <ShieldCheck className="w-4 h-4 text-green-500/50 mr-2" />
                      <span className="text-[10px] font-mono text-green-500/50 uppercase">No anomalies detected</span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default HealthDashboard;
