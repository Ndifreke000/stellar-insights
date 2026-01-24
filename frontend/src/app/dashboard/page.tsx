"use client"

import React, { useEffect, useState, useCallback } from 'react'
import {
  LineChart,
  Line,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
  Legend,
} from 'recharts'
import { SkeletonMetricsCard } from '@/components/ui/Skeleton'

type Corridor = {
  id: string
  health: number
  successRate: number
}

type TopAsset = {
  asset: string
  volume: number
  tvl: number
}

type TimePoint = {
  ts: string
  successRate: number
  settlementMs: number
  tvl: number
}

type DashboardData = {
  totalSuccessRate: number
  activeCorridors: Corridor[]
  topAssets: TopAsset[]
  timeseries: TimePoint[]
}

export default function DashboardPage() {
  const [data, setData] = useState<DashboardData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetchData = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await fetch('/api/dashboard')
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const json = await res.json()
      setData(json)
    } catch (err: any) {
      setError(err.message || 'Failed to load')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchData()
    const id = setInterval(fetchData, 30_000) // refresh every 30s
    return () => clearInterval(id)
  }, [fetchData])

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-semibold">Network Dashboard</h1>
        <div className="flex gap-2 items-center">
          <button
            className="px-3 py-1 rounded bg-sky-600 text-white text-sm"
            onClick={() => fetchData()}
            disabled={loading}
          >
            Refresh
          </button>
        </div>
      </div>

      {loading && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <SkeletonMetricsCard className="col-span-1" />
          <SkeletonMetricsCard className="col-span-1 lg:col-span-2" />
          <SkeletonMetricsCard className="col-span-1 lg:col-span-2" />
          <SkeletonMetricsCard className="col-span-1" />
          <SkeletonMetricsCard className="col-span-1 lg:col-span-2" />
        </div>
      )}

      {error && (
        <div className="rounded p-4 bg-rose-50 text-rose-700">Error: {error}</div>
      )}

      {data && (
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <div className="col-span-1 bg-white rounded shadow p-4">
            <h2 className="text-sm text-gray-500">Total Payment Success Rate</h2>
            <div className="mt-3 flex items-end gap-4">
              <div className="text-4xl font-bold">
                {(data.totalSuccessRate * 100).toFixed(2)}%
              </div>
              <div className="text-sm text-gray-500">(last 24h)</div>
            </div>
          </div>

          <div className="col-span-1 lg:col-span-2 bg-white rounded shadow p-4">
            <h2 className="text-sm text-gray-500">Settlement Speed (ms) — last 24 points</h2>
            <div style={{ width: '100%', height: 220 }} className="mt-3">
              <ResponsiveContainer>
                <LineChart data={data.timeseries}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="ts" tickFormatter={(s) => new Date(s).getHours() + ':00'} />
                  <YAxis />
                  <Tooltip labelFormatter={(s) => new Date(s).toLocaleString()} />
                  <Legend />
                  <Line type="monotone" dataKey="settlementMs" stroke="#8884d8" dot={false} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>

          <div className="col-span-1 lg:col-span-2 bg-white rounded shadow p-4">
            <h2 className="text-sm text-gray-500">Liquidity Depth / TVL (24h)</h2>
            <div style={{ width: '100%', height: 240 }} className="mt-3">
              <ResponsiveContainer>
                <LineChart data={data.timeseries}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="ts" tickFormatter={(s) => new Date(s).getHours() + ':00'} />
                  <YAxis />
                  <Tooltip labelFormatter={(s) => new Date(s).toLocaleString()} />
                  <Legend />
                  <Line type="monotone" dataKey="tvl" stroke="#82ca9d" dot={false} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>

          <div className="col-span-1 bg-white rounded shadow p-4">
            <h2 className="text-sm text-gray-500">Active Corridor Health</h2>
            <ul className="mt-3 space-y-3">
              {data.activeCorridors.map((c) => (
                <li key={c.id} className="flex items-center justify-between">
                  <div>
                    <div className="font-medium">{c.id}</div>
                    <div className="text-sm text-gray-500">Success: {(c.successRate * 100).toFixed(2)}%</div>
                  </div>
                  <div className="text-sm font-semibold">
                    {(c.health * 100).toFixed(0)}%
                  </div>
                </li>
              ))}
            </ul>
          </div>

          <div className="col-span-1 lg:col-span-2 bg-white rounded shadow p-4">
            <h2 className="text-sm text-gray-500">Top-performing Assets</h2>
            <div className="mt-3 overflow-auto">
              <table className="w-full text-left text-sm">
                <thead className="text-gray-500 text-xs uppercase">
                  <tr>
                    <th className="pb-2">Asset</th>
                    <th className="pb-2">Volume</th>
                    <th className="pb-2">TVL</th>
                  </tr>
                </thead>
                <tbody>
                  {data.topAssets.map((a) => (
                    <tr key={a.asset} className="border-t">
                      <td className="py-2 font-medium">{a.asset}</td>
                      <td className="py-2">{a.volume.toLocaleString()}</td>
                      <td className="py-2">${a.tvl.toLocaleString()}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
