/**
 * Portal Dashboard Page
 *
 * Self-service portal main page showing usage, subscription, and quick actions.
 */

import React from 'react';
import { Link } from 'react-router-dom';
import { format } from 'date-fns';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

// Mock data
const usageData = Array.from({ length: 7 }, (_, i) => ({
  date: format(new Date(Date.now() - (6 - i) * 24 * 60 * 60 * 1000), 'yyyy-MM-dd'),
  apiCalls: Math.floor(Math.random() * 5000) + 10000,
  tokens: Math.floor(Math.random() * 500000) + 2000000,
}));

const quickActions = [
  { title: 'API Keys', description: 'Manage your API keys', path: '/api-keys', icon: '🔑' },
  { title: 'Usage', description: 'View detailed usage', path: '/usage', icon: '📊' },
  { title: 'Billing', description: 'Manage subscription', path: '/billing', icon: '💳' },
  { title: 'Support', description: 'Get help', path: '/support', icon: '💬' },
];

export function Dashboard() {
  const formatNumber = (num: number) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Dashboard</h1>
        <p className="text-gray-500">Welcome back! Here's your account overview.</p>
      </div>

      {/* Subscription Status */}
      <div className="card bg-gradient-to-r from-primary-600 to-primary-700 text-white">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-primary-100">Current Plan</p>
            <p className="text-2xl font-bold">Professional</p>
            <p className="text-primary-200 text-sm mt-1">
              Renews on February 1, 2024
            </p>
          </div>
          <Link
            to="/billing"
            className="px-4 py-2 bg-white text-primary-600 rounded-lg font-medium hover:bg-primary-50 transition-colors"
          >
            Manage Plan
          </Link>
        </div>
      </div>

      {/* Usage Summary */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div className="card">
          <p className="text-sm text-gray-500">API Calls (This Month)</p>
          <p className="text-2xl font-bold">78,420</p>
          <div className="mt-2">
            <div className="flex justify-between text-xs text-gray-500 mb-1">
              <span>78% of limit</span>
              <span>100,000</span>
            </div>
            <div className="w-full bg-gray-200 rounded-full h-2">
              <div className="bg-primary-500 h-2 rounded-full" style={{ width: '78%' }} />
            </div>
          </div>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Input Tokens</p>
          <p className="text-2xl font-bold">{formatNumber(7850000)}</p>
          <div className="mt-2">
            <div className="flex justify-between text-xs text-gray-500 mb-1">
              <span>78% of limit</span>
              <span>10M</span>
            </div>
            <div className="w-full bg-gray-200 rounded-full h-2">
              <div className="bg-green-500 h-2 rounded-full" style={{ width: '78%' }} />
            </div>
          </div>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Output Tokens</p>
          <p className="text-2xl font-bold">{formatNumber(3920000)}</p>
          <div className="mt-2">
            <div className="flex justify-between text-xs text-gray-500 mb-1">
              <span>78% of limit</span>
              <span>5M</span>
            </div>
            <div className="w-full bg-gray-200 rounded-full h-2">
              <div className="bg-purple-500 h-2 rounded-full" style={{ width: '78%' }} />
            </div>
          </div>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Storage Used</p>
          <p className="text-2xl font-bold">45.2 GB</p>
          <div className="mt-2">
            <div className="flex justify-between text-xs text-gray-500 mb-1">
              <span>45% of limit</span>
              <span>100 GB</span>
            </div>
            <div className="w-full bg-gray-200 rounded-full h-2">
              <div className="bg-orange-500 h-2 rounded-full" style={{ width: '45%' }} />
            </div>
          </div>
        </div>
      </div>

      {/* Usage Chart */}
      <div className="card">
        <h3 className="text-lg font-semibold mb-4">Usage (Last 7 Days)</h3>
        <ResponsiveContainer width="100%" height={250}>
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
            <Area
              type="monotone"
              dataKey="apiCalls"
              stroke="#0EA5E9"
              fill="#0EA5E9"
              fillOpacity={0.2}
              name="API Calls"
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      {/* Quick Actions */}
      <div>
        <h3 className="text-lg font-semibold mb-4">Quick Actions</h3>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {quickActions.map((action) => (
            <Link
              key={action.path}
              to={action.path}
              className="card hover:shadow-md transition-shadow flex items-center space-x-4"
            >
              <span className="text-3xl">{action.icon}</span>
              <div>
                <p className="font-medium">{action.title}</p>
                <p className="text-sm text-gray-500">{action.description}</p>
              </div>
            </Link>
          ))}
        </div>
      </div>

      {/* Recent Activity */}
      <div className="card">
        <h3 className="text-lg font-semibold mb-4">Recent Activity</h3>
        <div className="space-y-4">
          {[
            { action: 'API key created', details: 'Production key: sk-prod-...', time: '2 hours ago' },
            { action: 'Invoice paid', details: 'Invoice #INV-2024-0012', time: '3 days ago' },
            { action: 'Usage alert', details: 'API calls reached 75%', time: '5 days ago' },
            { action: 'Plan upgraded', details: 'Starter → Professional', time: '1 week ago' },
          ].map((activity, index) => (
            <div key={index} className="flex items-center justify-between py-2 border-b border-gray-100 last:border-0">
              <div>
                <p className="font-medium">{activity.action}</p>
                <p className="text-sm text-gray-500">{activity.details}</p>
              </div>
              <span className="text-sm text-gray-400">{activity.time}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export default Dashboard;
