'use client';

import React, { useState, useMemo } from 'react';
import { LineChart, Line, BarChart, Bar, PieChart, Pie, Cell, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';

interface DataPoint {
  id?: string | number;
  name: string;
  value: number;
  [key: string]: string | number | undefined;
}

interface AdvancedDataVisualizationProps {
  data: DataPoint[];
  title?: string;
  chartType?: 'line' | 'bar' | 'pie';
  xAxisKey?: string;
  yAxisKey?: string;
  colors?: string[];
  responsive?: boolean;
  height?: number;
}

const COLORS = ['#0088FE', '#00C49F', '#FFBB28', '#FF8042', '#8884D8', '#82CA9D', '#FFC658', '#FF7C7C'];

export const AdvancedDataVisualization: React.FC<AdvancedDataVisualizationProps> = ({
  data,
  title = 'Data Visualization',
  chartType = 'line',
  xAxisKey = 'name',
  yAxisKey = 'value',
  colors = COLORS,
  responsive = true,
  height = 400,
}) => {
  const [selectedMetric, setSelectedMetric] = useState<string>(yAxisKey);
  const [isLoading, setIsLoading] = useState(false);

  const metrics = useMemo(() => {
    if (data.length === 0) return [];
    return Object.keys(data[0]).filter(key => key !== xAxisKey && typeof data[0][key] === 'number');
  }, [data, xAxisKey]);

  const chartProps = {
    data,
    margin: { top: 5, right: 30, left: 0, bottom: 5 },
  };

  const renderChart = () => {
    switch (chartType) {
      case 'bar':
        return (
          <ResponsiveContainer width="100%" height={height}>
            <BarChart {...chartProps}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey={xAxisKey} />
              <YAxis />
              <Tooltip />
              <Legend />
              <Bar dataKey={selectedMetric} fill={colors[0]} />
            </BarChart>
          </ResponsiveContainer>
        );
      case 'pie':
        return (
          <ResponsiveContainer width="100%" height={height}>
            <PieChart>
              <Pie
                data={data}
                cx="50%"
                cy="50%"
                labelLine={false}
                label={({ name, value }) => `${name}: ${value}`}
                outerRadius={80}
                fill="#8884d8"
                dataKey={selectedMetric}
              >
                {data.map((_, index) => (
                  <Cell key={`cell-${index}`} fill={colors[index % colors.length]} />
                ))}
              </Pie>
              <Tooltip />
            </PieChart>
          </ResponsiveContainer>
        );
      case 'line':
      default:
        return (
          <ResponsiveContainer width="100%" height={height}>
            <LineChart {...chartProps}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey={xAxisKey} />
              <YAxis />
              <Tooltip />
              <Legend />
              <Line
                type="monotone"
                dataKey={selectedMetric}
                stroke={colors[0]}
                dot={{ fill: colors[0], r: 4 }}
                activeDot={{ r: 6 }}
              />
            </LineChart>
          </ResponsiveContainer>
        );
    }
  };

  return (
    <div className="w-full bg-white rounded-lg shadow-lg p-6" role="region" aria-label={title}>
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-800 mb-4">{title}</h2>

        {metrics.length > 1 && (
          <div className="flex flex-wrap gap-2 mb-4">
            <label htmlFor="metric-select" className="text-sm font-medium text-gray-700">
              Select Metric:
            </label>
            <select
              id="metric-select"
              value={selectedMetric}
              onChange={(e) => setSelectedMetric(e.target.value)}
              className="px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
              aria-label="Select metric to display"
            >
              {metrics.map((metric) => (
                <option key={metric} value={metric}>
                  {metric}
                </option>
              ))}
            </select>
          </div>
        )}

        {isLoading && (
          <div className="flex items-center justify-center h-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
          </div>
        )}
      </div>

      <div className="overflow-x-auto">
        {data.length > 0 ? (
          renderChart()
        ) : (
          <div className="flex items-center justify-center h-96 text-gray-500">
            <p>No data available to display</p>
          </div>
        )}
      </div>

      <div className="mt-6 grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="bg-blue-50 p-4 rounded-lg">
          <p className="text-sm text-gray-600">Total Points</p>
          <p className="text-2xl font-bold text-blue-600">{data.length}</p>
        </div>
        {metrics.length > 0 && (
          <div className="bg-green-50 p-4 rounded-lg">
            <p className="text-sm text-gray-600">Metrics</p>
            <p className="text-2xl font-bold text-green-600">{metrics.length}</p>
          </div>
        )}
        {data.length > 0 && (
          <>
            <div className="bg-purple-50 p-4 rounded-lg">
              <p className="text-sm text-gray-600">Max Value</p>
              <p className="text-2xl font-bold text-purple-600">
                {Math.max(...data.map(d => (d[selectedMetric] as number) || 0))}
              </p>
            </div>
            <div className="bg-orange-50 p-4 rounded-lg">
              <p className="text-sm text-gray-600">Avg Value</p>
              <p className="text-2xl font-bold text-orange-600">
                {(data.reduce((sum, d) => sum + ((d[selectedMetric] as number) || 0), 0) / data.length).toFixed(2)}
              </p>
            </div>
          </>
        )}
      </div>
    </div>
  );
};

export default AdvancedDataVisualization;
