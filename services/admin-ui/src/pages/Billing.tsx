/**
 * Billing Page
 *
 * Invoice management and revenue overview.
 */

import React, { useState } from 'react';
import { format } from 'date-fns';
import { clsx } from 'clsx';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';
import type { Invoice, InvoiceStatus } from '../types';

// Mock data
const mockInvoices: Invoice[] = [
  {
    id: '1',
    tenantId: '1',
    number: 'INV-2024-0001',
    status: 'paid',
    total: 4500,
    amountPaid: 4500,
    amountDue: 0,
    periodStart: '2024-01-01T00:00:00Z',
    periodEnd: '2024-01-31T23:59:59Z',
    paidAt: '2024-01-05T10:30:00Z',
    createdAt: '2024-01-01T00:00:00Z',
  },
  {
    id: '2',
    tenantId: '2',
    number: 'INV-2024-0002',
    status: 'paid',
    total: 2970,
    amountPaid: 2970,
    amountDue: 0,
    periodStart: '2024-01-01T00:00:00Z',
    periodEnd: '2024-01-31T23:59:59Z',
    paidAt: '2024-01-03T14:20:00Z',
    createdAt: '2024-01-01T00:00:00Z',
  },
  {
    id: '3',
    tenantId: '3',
    number: 'INV-2024-0003',
    status: 'open',
    total: 2180,
    amountPaid: 0,
    amountDue: 2180,
    periodStart: '2024-01-01T00:00:00Z',
    periodEnd: '2024-01-31T23:59:59Z',
    dueDate: '2024-02-15T23:59:59Z',
    createdAt: '2024-01-01T00:00:00Z',
  },
  {
    id: '4',
    tenantId: '4',
    number: 'INV-2024-0004',
    status: 'paid',
    total: 1890,
    amountPaid: 1890,
    amountDue: 0,
    periodStart: '2024-01-01T00:00:00Z',
    periodEnd: '2024-01-31T23:59:59Z',
    paidAt: '2024-01-08T09:15:00Z',
    createdAt: '2024-01-01T00:00:00Z',
  },
  {
    id: '5',
    tenantId: '5',
    number: 'INV-2024-0005',
    status: 'void',
    total: 290,
    amountPaid: 0,
    amountDue: 0,
    periodStart: '2024-01-01T00:00:00Z',
    periodEnd: '2024-01-31T23:59:59Z',
    createdAt: '2024-01-01T00:00:00Z',
  },
];

const revenueData = [
  { month: '2023-07', revenue: 65000 },
  { month: '2023-08', revenue: 68500 },
  { month: '2023-09', revenue: 72000 },
  { month: '2023-10', revenue: 75800 },
  { month: '2023-11', revenue: 82000 },
  { month: '2023-12', revenue: 85500 },
  { month: '2024-01', revenue: 89500 },
];

const tenantNames: Record<string, string> = {
  '1': 'Acme Corporation',
  '2': 'TechStart Inc',
  '3': 'DataFlow Labs',
  '4': 'CloudNative Co',
  '5': 'StartupXYZ',
};

export function Billing() {
  const [filterStatus, setFilterStatus] = useState<string>('all');

  const formatCurrency = (amount: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
    }).format(amount);
  };

  const getStatusBadgeClass = (status: InvoiceStatus) => {
    switch (status) {
      case 'paid':
        return 'badge-success';
      case 'open':
        return 'badge-info';
      case 'void':
        return 'bg-gray-100 text-gray-600';
      case 'uncollectible':
        return 'badge-danger';
      default:
        return 'badge-warning';
    }
  };

  const filteredInvoices = mockInvoices.filter((invoice) =>
    filterStatus === 'all' ? true : invoice.status === filterStatus
  );

  const totalRevenue = revenueData.reduce((sum, d) => sum + d.revenue, 0);
  const currentMonthRevenue = revenueData[revenueData.length - 1].revenue;
  const lastMonthRevenue = revenueData[revenueData.length - 2].revenue;
  const revenueGrowth = ((currentMonthRevenue - lastMonthRevenue) / lastMonthRevenue) * 100;

  const openInvoicesTotal = mockInvoices
    .filter((i) => i.status === 'open')
    .reduce((sum, i) => sum + i.amountDue, 0);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Billing</h1>
          <p className="text-gray-500">Invoice management and revenue overview</p>
        </div>
        <button className="btn btn-primary">
          <svg className="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
          Export Report
        </button>
      </div>

      {/* Revenue Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="card">
          <p className="text-sm text-gray-500">Monthly Revenue</p>
          <p className="text-2xl font-bold">{formatCurrency(currentMonthRevenue)}</p>
          <p className={`text-sm ${revenueGrowth >= 0 ? 'text-green-600' : 'text-red-600'}`}>
            {revenueGrowth >= 0 ? '+' : ''}{revenueGrowth.toFixed(1)}% vs last month
          </p>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Total Revenue (YTD)</p>
          <p className="text-2xl font-bold">{formatCurrency(totalRevenue)}</p>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Outstanding</p>
          <p className="text-2xl font-bold text-orange-600">{formatCurrency(openInvoicesTotal)}</p>
          <p className="text-sm text-gray-500">
            {mockInvoices.filter((i) => i.status === 'open').length} invoices
          </p>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Collection Rate</p>
          <p className="text-2xl font-bold text-green-600">98.2%</p>
          <p className="text-sm text-gray-500">Last 90 days</p>
        </div>
      </div>

      {/* Revenue Chart */}
      <div className="card">
        <h3 className="text-lg font-semibold mb-4">Revenue Trend</h3>
        <ResponsiveContainer width="100%" height={300}>
          <AreaChart data={revenueData}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis
              dataKey="month"
              tickFormatter={(month) => format(new Date(month + '-01'), 'MMM yyyy')}
            />
            <YAxis tickFormatter={(value) => `$${(value / 1000).toFixed(0)}k`} />
            <Tooltip
              labelFormatter={(month) => format(new Date(month + '-01'), 'MMMM yyyy')}
              formatter={(value: number) => [formatCurrency(value), 'Revenue']}
            />
            <Area
              type="monotone"
              dataKey="revenue"
              stroke="#10B981"
              fill="#10B981"
              fillOpacity={0.2}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      {/* Invoices */}
      <div className="card">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold">Recent Invoices</h3>
          <select
            className="input w-auto"
            value={filterStatus}
            onChange={(e) => setFilterStatus(e.target.value)}
          >
            <option value="all">All Status</option>
            <option value="open">Open</option>
            <option value="paid">Paid</option>
            <option value="void">Void</option>
            <option value="uncollectible">Uncollectible</option>
          </select>
        </div>
        <div className="overflow-x-auto">
          <table className="table">
            <thead>
              <tr>
                <th>Invoice</th>
                <th>Tenant</th>
                <th>Status</th>
                <th>Amount</th>
                <th>Due Date</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {filteredInvoices.map((invoice) => (
                <tr key={invoice.id} className="hover:bg-gray-50">
                  <td>
                    <div>
                      <p className="font-medium">{invoice.number}</p>
                      <p className="text-sm text-gray-500">
                        {format(new Date(invoice.createdAt), 'MMM d, yyyy')}
                      </p>
                    </div>
                  </td>
                  <td>{tenantNames[invoice.tenantId] || 'Unknown'}</td>
                  <td>
                    <span className={clsx('badge capitalize', getStatusBadgeClass(invoice.status))}>
                      {invoice.status}
                    </span>
                  </td>
                  <td>
                    <div>
                      <p className="font-medium">{formatCurrency(invoice.total)}</p>
                      {invoice.amountDue > 0 && (
                        <p className="text-sm text-orange-600">
                          {formatCurrency(invoice.amountDue)} due
                        </p>
                      )}
                    </div>
                  </td>
                  <td>
                    {invoice.paidAt ? (
                      <span className="text-green-600">
                        Paid {format(new Date(invoice.paidAt), 'MMM d')}
                      </span>
                    ) : invoice.dueDate ? (
                      format(new Date(invoice.dueDate), 'MMM d, yyyy')
                    ) : (
                      '-'
                    )}
                  </td>
                  <td>
                    <div className="flex space-x-2">
                      <button className="p-2 hover:bg-gray-100 rounded-lg" title="View">
                        <svg className="w-4 h-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                        </svg>
                      </button>
                      <button className="p-2 hover:bg-gray-100 rounded-lg" title="Download PDF">
                        <svg className="w-4 h-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                        </svg>
                      </button>
                      {invoice.status === 'open' && (
                        <button className="p-2 hover:bg-gray-100 rounded-lg" title="Send Reminder">
                          <svg className="w-4 h-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                          </svg>
                        </button>
                      )}
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

export default Billing;
