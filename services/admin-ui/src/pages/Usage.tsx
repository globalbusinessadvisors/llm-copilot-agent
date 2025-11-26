/**
 * Usage Page
 *
 * Platform-wide usage analytics and metrics.
 */

import React, { useState } from 'react';
import { format, subDays } from 'date-fns';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';

// Mock data
const usageData = Array.from({ length: 30 }, (_, i) => {
  const date = subDays(new Date(), 29 - i);
  return {
    date: format(date, 'yyyy-MM-dd'),
    apiCalls: Math.floor(Math.random() * 50000) + 100000,
    inputTokens: Math.floor(Math.random() * 5000000) + 20000000,
    outputTokens: Math.floor(Math.random() * 2000000) + 10000000,
    computeHours: Math.floor(Math.random() * 100) + 200,
    storageGb: Math.floor(Math.random() * 50) + 450,
  };
});

const topTenants = [
  { name: 'Acme Corp', apiCalls: 245000, tokens: 15000000, revenue: 4500 },
  { name: 'TechStart Inc', apiCalls: 189000, tokens: 12000000, revenue: 2970 },
  { name: 'DataFlow Labs', apiCalls: 156000, tokens: 9800000, revenue: 2180 },
  { name: 'CloudNative Co', apiCalls: 134000, tokens: 8500000, revenue: 1890 },
  { name: 'AI Solutions', apiCalls: 98000, tokens: 6200000, revenue: 1470 },
];

const modelUsage = [
  { model: 'claude-3-opus', calls: 45000, tokens: 8500000 },
  { model: 'claude-3-sonnet', calls: 120000, tokens: 15000000 },
  { model: 'claude-3-haiku', calls: 250000, tokens: 12000000 },
  { model: 'claude-2', calls: 25000, tokens: 3500000 },
];

export function Usage() {
  const [dateRange, setDateRange] = useState('30d');
  const [metric, setMetric] = useState<'apiCalls' | 'tokens' | 'compute'>('apiCalls');

  const formatNumber = (num: number) => {
    if (num >= 1000000000) return `${(num / 1000000000).toFixed(1)}B`;
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  const totalApiCalls = usageData.reduce((sum, d) => sum + d.apiCalls, 0);
  const totalInputTokens = usageData.reduce((sum, d) => sum + d.inputTokens, 0);
  const totalOutputTokens = usageData.reduce((sum, d) => sum + d.outputTokens, 0);
  const avgComputeHours = usageData.reduce((sum, d) => sum + d.computeHours, 0) / usageData.length;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Usage Analytics</h1>
          <p className="text-gray-500">Platform-wide usage metrics and trends</p>
        </div>
        <div className="flex space-x-3">
          <select
            className="input w-auto"
            value={dateRange}
            onChange={(e) => setDateRange(e.target.value)}
          >
            <option value="7d">Last 7 days</option>
            <option value="30d">Last 30 days</option>
            <option value="90d">Last 90 days</option>
          </select>
          <button className="btn btn-secondary">Export</button>
        </div>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="card">
          <p className="text-sm text-gray-500">Total API Calls</p>
          <p className="text-2xl font-bold">{formatNumber(totalApiCalls)}</p>
          <p className="text-sm text-green-600">+12% vs last period</p>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Input Tokens</p>
          <p className="text-2xl font-bold">{formatNumber(totalInputTokens)}</p>
          <p className="text-sm text-green-600">+8% vs last period</p>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Output Tokens</p>
          <p className="text-2xl font-bold">{formatNumber(totalOutputTokens)}</p>
          <p className="text-sm text-green-600">+15% vs last period</p>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Avg Compute Hours/Day</p>
          <p className="text-2xl font-bold">{avgComputeHours.toFixed(0)}</p>
          <p className="text-sm text-green-600">+5% vs last period</p>
        </div>
      </div>

      {/* Main Chart */}
      <div className="card">
        <div className="flex items-center justify-between mb-6">
          <h3 className="text-lg font-semibold">Usage Trend</h3>
          <div className="flex space-x-2">
            {(['apiCalls', 'tokens', 'compute'] as const).map((m) => (
              <button
                key={m}
                onClick={() => setMetric(m)}
                className={`px-3 py-1 rounded-lg text-sm ${
                  metric === m
                    ? 'bg-primary-100 text-primary-700'
                    : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                }`}
              >
                {m === 'apiCalls' ? 'API Calls' : m === 'tokens' ? 'Tokens' : 'Compute'}
              </button>
            ))}
          </div>
        </div>
        <ResponsiveContainer width="100%" height={350}>
          <AreaChart data={usageData}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis
              dataKey="date"
              tickFormatter={(date) => format(new Date(date), 'MMM d')}
            />
            <YAxis tickFormatter={formatNumber} />
            <Tooltip
              labelFormatter={(date) => format(new Date(date as string), 'MMM d, yyyy')}
              formatter={(value: number) => [formatNumber(value), '']}
            />
            {metric === 'apiCalls' && (
              <Area
                type="monotone"
                dataKey="apiCalls"
                stroke="#0EA5E9"
                fill="#0EA5E9"
                fillOpacity={0.2}
                name="API Calls"
              />
            )}
            {metric === 'tokens' && (
              <>
                <Area
                  type="monotone"
                  dataKey="inputTokens"
                  stroke="#10B981"
                  fill="#10B981"
                  fillOpacity={0.2}
                  name="Input Tokens"
                />
                <Area
                  type="monotone"
                  dataKey="outputTokens"
                  stroke="#8B5CF6"
                  fill="#8B5CF6"
                  fillOpacity={0.2}
                  name="Output Tokens"
                />
              </>
            )}
            {metric === 'compute' && (
              <Area
                type="monotone"
                dataKey="computeHours"
                stroke="#F59E0B"
                fill="#F59E0B"
                fillOpacity={0.2}
                name="Compute Hours"
              />
            )}
            <Legend />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Top Tenants */}
        <div className="card">
          <h3 className="text-lg font-semibold mb-4">Top Tenants by Usage</h3>
          <div className="space-y-4">
            {topTenants.map((tenant, index) => (
              <div
                key={tenant.name}
                className="flex items-center justify-between py-2 border-b border-gray-100 last:border-0"
              >
                <div className="flex items-center space-x-3">
                  <span className="text-sm text-gray-400 w-6">{index + 1}</span>
                  <div>
                    <p className="font-medium">{tenant.name}</p>
                    <p className="text-sm text-gray-500">
                      {formatNumber(tenant.apiCalls)} calls
                    </p>
                  </div>
                </div>
                <div className="text-right">
                  <p className="font-medium">{formatNumber(tenant.tokens)}</p>
                  <p className="text-sm text-gray-500">tokens</p>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Model Usage */}
        <div className="card">
          <h3 className="text-lg font-semibold mb-4">Usage by Model</h3>
          <ResponsiveContainer width="100%" height={250}>
            <BarChart data={modelUsage} layout="vertical">
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis type="number" tickFormatter={formatNumber} />
              <YAxis type="category" dataKey="model" width={120} />
              <Tooltip formatter={(value: number) => formatNumber(value)} />
              <Bar dataKey="calls" fill="#0EA5E9" name="API Calls" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Usage by Type */}
      <div className="card">
        <h3 className="text-lg font-semibold mb-4">Usage Breakdown by Type</h3>
        <div className="overflow-x-auto">
          <table className="table">
            <thead>
              <tr>
                <th>Usage Type</th>
                <th>This Period</th>
                <th>Last Period</th>
                <th>Change</th>
                <th>% of Total</th>
              </tr>
            </thead>
            <tbody>
              {[
                { type: 'API Calls', current: 4250000, previous: 3800000, percent: 45 },
                { type: 'Input Tokens', current: 850000000, previous: 780000000, percent: 28 },
                { type: 'Output Tokens', current: 420000000, previous: 365000000, percent: 22 },
                { type: 'Embeddings', current: 125000000, previous: 110000000, percent: 5 },
              ].map((row) => (
                <tr key={row.type}>
                  <td className="font-medium">{row.type}</td>
                  <td>{formatNumber(row.current)}</td>
                  <td>{formatNumber(row.previous)}</td>
                  <td>
                    <span
                      className={
                        row.current > row.previous ? 'text-green-600' : 'text-red-600'
                      }
                    >
                      {row.current > row.previous ? '+' : ''}
                      {(((row.current - row.previous) / row.previous) * 100).toFixed(1)}%
                    </span>
                  </td>
                  <td>
                    <div className="flex items-center space-x-2">
                      <div className="flex-1 bg-gray-200 rounded-full h-2">
                        <div
                          className="bg-primary-500 h-2 rounded-full"
                          style={{ width: `${row.percent}%` }}
                        />
                      </div>
                      <span className="text-sm text-gray-500">{row.percent}%</span>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

export default Usage;
