/**
 * Dashboard Page
 *
 * Main overview page showing key metrics and system health.
 */

import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { format } from 'date-fns';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from 'recharts';
import api from '../services/api';
import type { DashboardStats, ServiceHealth } from '../types';

// Mock data for charts (in production, fetch from API)
const usageData = [
  { date: '2024-01-01', apiCalls: 12400, tokens: 2400000 },
  { date: '2024-01-02', apiCalls: 13300, tokens: 2210000 },
  { date: '2024-01-03', apiCalls: 15100, tokens: 2900000 },
  { date: '2024-01-04', apiCalls: 14800, tokens: 2700000 },
  { date: '2024-01-05', apiCalls: 16200, tokens: 3100000 },
  { date: '2024-01-06', apiCalls: 14900, tokens: 2850000 },
  { date: '2024-01-07', apiCalls: 17500, tokens: 3400000 },
];

const planDistribution = [
  { name: 'Free', value: 450, color: '#9CA3AF' },
  { name: 'Starter', value: 280, color: '#60A5FA' },
  { name: 'Professional', value: 150, color: '#34D399' },
  { name: 'Enterprise', value: 45, color: '#A78BFA' },
];

interface StatCardProps {
  title: string;
  value: string | number;
  change?: number;
  icon: React.ReactNode;
  color: 'blue' | 'green' | 'purple' | 'orange';
}

function StatCard({ title, value, change, icon, color }: StatCardProps) {
  const colorClasses = {
    blue: 'bg-blue-50 text-blue-600',
    green: 'bg-green-50 text-green-600',
    purple: 'bg-purple-50 text-purple-600',
    orange: 'bg-orange-50 text-orange-600',
  };

  return (
    <div className="card">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-gray-500 mb-1">{title}</p>
          <p className="text-2xl font-bold">{value}</p>
          {change !== undefined && (
            <p
              className={`text-sm mt-1 ${
                change >= 0 ? 'text-green-600' : 'text-red-600'
              }`}
            >
              {change >= 0 ? '+' : ''}
              {change}% from last month
            </p>
          )}
        </div>
        <div className={`p-3 rounded-lg ${colorClasses[color]}`}>{icon}</div>
      </div>
    </div>
  );
}

function ServiceHealthCard({ service }: { service: ServiceHealth }) {
  const statusColors = {
    healthy: 'bg-green-500',
    degraded: 'bg-yellow-500',
    down: 'bg-red-500',
  };

  return (
    <div className="flex items-center justify-between py-2">
      <div className="flex items-center space-x-3">
        <div className={`w-2 h-2 rounded-full ${statusColors[service.status]}`} />
        <span className="text-sm font-medium">{service.name}</span>
      </div>
      <span className="text-sm text-gray-500">{service.latency}ms</span>
    </div>
  );
}

export function Dashboard() {
  // In production, these would be real API calls
  const mockStats: DashboardStats = {
    totalTenants: 925,
    activeTenants: 856,
    totalRevenue: 1250000,
    monthlyRevenue: 89500,
    totalApiCalls: 15400000,
    totalTokens: 4200000000,
    systemHealth: {
      status: 'healthy',
      uptime: 99.98,
      services: [
        { name: 'API Gateway', status: 'healthy', latency: 45, lastCheck: new Date().toISOString() },
        { name: 'Auth Service', status: 'healthy', latency: 23, lastCheck: new Date().toISOString() },
        { name: 'Billing Service', status: 'healthy', latency: 67, lastCheck: new Date().toISOString() },
        { name: 'AI Service', status: 'healthy', latency: 156, lastCheck: new Date().toISOString() },
        { name: 'Database', status: 'healthy', latency: 12, lastCheck: new Date().toISOString() },
        { name: 'Redis Cache', status: 'healthy', latency: 5, lastCheck: new Date().toISOString() },
      ],
    },
  };

  const formatNumber = (num: number) => {
    if (num >= 1000000000) return `${(num / 1000000000).toFixed(1)}B`;
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  const formatCurrency = (amount: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Dashboard</h1>
          <p className="text-gray-500">Welcome back! Here's what's happening today.</p>
        </div>
        <div className="text-sm text-gray-500">
          Last updated: {format(new Date(), 'MMM d, yyyy h:mm a')}
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <StatCard
          title="Active Tenants"
          value={mockStats.activeTenants.toLocaleString()}
          change={12}
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
          }
          color="blue"
        />
        <StatCard
          title="Monthly Revenue"
          value={formatCurrency(mockStats.monthlyRevenue)}
          change={8}
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          }
          color="green"
        />
        <StatCard
          title="API Calls (30d)"
          value={formatNumber(mockStats.totalApiCalls)}
          change={23}
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          }
          color="purple"
        />
        <StatCard
          title="Tokens Used (30d)"
          value={formatNumber(mockStats.totalTokens)}
          change={18}
          icon={
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 7h6m0 10v-3m-3 3h.01M9 17h.01M9 14h.01M12 14h.01M15 11h.01M12 11h.01M9 11h.01M7 21h10a2 2 0 002-2V5a2 2 0 00-2-2H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
            </svg>
          }
          color="orange"
        />
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Usage Trend */}
        <div className="card">
          <h3 className="text-lg font-semibold mb-4">Usage Trend (7 Days)</h3>
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={usageData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis
                dataKey="date"
                tickFormatter={(date) => format(new Date(date), 'MMM d')}
              />
              <YAxis />
              <Tooltip
                labelFormatter={(date) => format(new Date(date as string), 'MMM d, yyyy')}
                formatter={(value: number) => formatNumber(value)}
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

        {/* Plan Distribution */}
        <div className="card">
          <h3 className="text-lg font-semibold mb-4">Plan Distribution</h3>
          <div className="flex items-center">
            <ResponsiveContainer width="60%" height={250}>
              <PieChart>
                <Pie
                  data={planDistribution}
                  cx="50%"
                  cy="50%"
                  innerRadius={60}
                  outerRadius={80}
                  paddingAngle={5}
                  dataKey="value"
                >
                  {planDistribution.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip />
              </PieChart>
            </ResponsiveContainer>
            <div className="flex-1 space-y-3">
              {planDistribution.map((plan) => (
                <div key={plan.name} className="flex items-center justify-between">
                  <div className="flex items-center space-x-2">
                    <div
                      className="w-3 h-3 rounded-full"
                      style={{ backgroundColor: plan.color }}
                    />
                    <span className="text-sm">{plan.name}</span>
                  </div>
                  <span className="text-sm font-medium">{plan.value}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* System Health & Recent Activity */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* System Health */}
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold">System Health</h3>
            <span
              className={`badge ${
                mockStats.systemHealth.status === 'healthy'
                  ? 'badge-success'
                  : mockStats.systemHealth.status === 'degraded'
                  ? 'badge-warning'
                  : 'badge-danger'
              }`}
            >
              {mockStats.systemHealth.status}
            </span>
          </div>
          <div className="mb-4">
            <p className="text-sm text-gray-500">Uptime</p>
            <p className="text-2xl font-bold text-green-600">
              {mockStats.systemHealth.uptime}%
            </p>
          </div>
          <div className="border-t pt-4 space-y-1">
            {mockStats.systemHealth.services.map((service) => (
              <ServiceHealthCard key={service.name} service={service} />
            ))}
          </div>
        </div>

        {/* Recent Activity */}
        <div className="card lg:col-span-2">
          <h3 className="text-lg font-semibold mb-4">Recent Activity</h3>
          <div className="space-y-4">
            {[
              { type: 'subscription', message: 'New enterprise subscription', tenant: 'Acme Corp', time: '5 min ago' },
              { type: 'ticket', message: 'Support ticket resolved', tenant: 'TechStart Inc', time: '15 min ago' },
              { type: 'usage', message: 'Quota warning at 80%', tenant: 'DataFlow Labs', time: '1 hour ago' },
              { type: 'incident', message: 'Incident resolved: API latency', tenant: 'System', time: '2 hours ago' },
              { type: 'subscription', message: 'Plan upgrade to Professional', tenant: 'CloudNative Co', time: '3 hours ago' },
            ].map((activity, index) => (
              <div key={index} className="flex items-start space-x-3 pb-4 border-b border-gray-100 last:border-0 last:pb-0">
                <div
                  className={`w-8 h-8 rounded-full flex items-center justify-center ${
                    activity.type === 'subscription'
                      ? 'bg-green-100'
                      : activity.type === 'ticket'
                      ? 'bg-blue-100'
                      : activity.type === 'usage'
                      ? 'bg-yellow-100'
                      : 'bg-red-100'
                  }`}
                >
                  {activity.type === 'subscription' && (
                    <svg className="w-4 h-4 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                    </svg>
                  )}
                  {activity.type === 'ticket' && (
                    <svg className="w-4 h-4 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z" />
                    </svg>
                  )}
                  {activity.type === 'usage' && (
                    <svg className="w-4 h-4 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                    </svg>
                  )}
                  {activity.type === 'incident' && (
                    <svg className="w-4 h-4 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20.618 5.984A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016zM12 9v2m0 4h.01" />
                    </svg>
                  )}
                </div>
                <div className="flex-1">
                  <p className="text-sm font-medium">{activity.message}</p>
                  <p className="text-xs text-gray-500">{activity.tenant}</p>
                </div>
                <span className="text-xs text-gray-400">{activity.time}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

export default Dashboard;
