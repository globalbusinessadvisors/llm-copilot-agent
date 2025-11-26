/**
 * Tenants Page
 *
 * Manage tenants/customers with CRUD operations.
 */

import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { format } from 'date-fns';
import { clsx } from 'clsx';
import type { Tenant, PlanType, TenantStatus } from '../types';

// Mock data
const mockTenants: Tenant[] = [
  {
    id: '1',
    name: 'Acme Corporation',
    email: 'admin@acme.com',
    planType: 'enterprise',
    status: 'active',
    stripeCustomerId: 'cus_123',
    createdAt: '2023-06-15T10:30:00Z',
    updatedAt: '2024-01-10T14:20:00Z',
  },
  {
    id: '2',
    name: 'TechStart Inc',
    email: 'billing@techstart.io',
    planType: 'professional',
    status: 'active',
    stripeCustomerId: 'cus_456',
    createdAt: '2023-08-20T08:15:00Z',
    updatedAt: '2024-01-08T11:45:00Z',
  },
  {
    id: '3',
    name: 'DataFlow Labs',
    email: 'team@dataflow.ai',
    planType: 'starter',
    status: 'active',
    stripeCustomerId: 'cus_789',
    createdAt: '2023-11-01T16:00:00Z',
    updatedAt: '2024-01-05T09:30:00Z',
  },
  {
    id: '4',
    name: 'CloudNative Co',
    email: 'hello@cloudnative.dev',
    planType: 'professional',
    status: 'active',
    stripeCustomerId: 'cus_101',
    createdAt: '2023-09-10T12:00:00Z',
    updatedAt: '2024-01-02T17:15:00Z',
  },
  {
    id: '5',
    name: 'StartupXYZ',
    email: 'info@startupxyz.com',
    planType: 'free',
    status: 'suspended',
    createdAt: '2023-12-01T09:00:00Z',
    updatedAt: '2024-01-01T10:00:00Z',
  },
];

interface TenantModalProps {
  tenant: Tenant | null;
  onClose: () => void;
  onSave: (tenant: Partial<Tenant>) => void;
}

function TenantModal({ tenant, onClose, onSave }: TenantModalProps) {
  const [formData, setFormData] = useState({
    name: tenant?.name || '',
    email: tenant?.email || '',
    planType: tenant?.planType || 'free',
    status: tenant?.status || 'active',
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSave(formData);
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-xl max-w-md w-full p-6">
        <h2 className="text-xl font-bold mb-4">
          {tenant ? 'Edit Tenant' : 'Add Tenant'}
        </h2>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="label">Name</label>
            <input
              type="text"
              className="input"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              required
            />
          </div>
          <div>
            <label className="label">Email</label>
            <input
              type="email"
              className="input"
              value={formData.email}
              onChange={(e) => setFormData({ ...formData, email: e.target.value })}
              required
            />
          </div>
          <div>
            <label className="label">Plan</label>
            <select
              className="input"
              value={formData.planType}
              onChange={(e) =>
                setFormData({ ...formData, planType: e.target.value as PlanType })
              }
            >
              <option value="free">Free</option>
              <option value="starter">Starter</option>
              <option value="professional">Professional</option>
              <option value="enterprise">Enterprise</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <div>
            <label className="label">Status</label>
            <select
              className="input"
              value={formData.status}
              onChange={(e) =>
                setFormData({ ...formData, status: e.target.value as TenantStatus })
              }
            >
              <option value="active">Active</option>
              <option value="suspended">Suspended</option>
              <option value="deleted">Deleted</option>
            </select>
          </div>
          <div className="flex space-x-3 pt-4">
            <button type="button" onClick={onClose} className="btn btn-secondary flex-1">
              Cancel
            </button>
            <button type="submit" className="btn btn-primary flex-1">
              Save
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

export function Tenants() {
  const [selectedTenant, setSelectedTenant] = useState<Tenant | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterPlan, setFilterPlan] = useState<string>('all');
  const [filterStatus, setFilterStatus] = useState<string>('all');

  const tenants = mockTenants;

  const filteredTenants = tenants.filter((tenant) => {
    const matchesSearch =
      tenant.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      tenant.email.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesPlan = filterPlan === 'all' || tenant.planType === filterPlan;
    const matchesStatus = filterStatus === 'all' || tenant.status === filterStatus;
    return matchesSearch && matchesPlan && matchesStatus;
  });

  const getPlanBadgeClass = (plan: PlanType) => {
    switch (plan) {
      case 'enterprise':
        return 'bg-purple-100 text-purple-800';
      case 'professional':
        return 'bg-green-100 text-green-800';
      case 'starter':
        return 'bg-blue-100 text-blue-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const getStatusBadgeClass = (status: TenantStatus) => {
    switch (status) {
      case 'active':
        return 'badge-success';
      case 'suspended':
        return 'badge-warning';
      case 'deleted':
        return 'badge-danger';
      default:
        return 'badge-info';
    }
  };

  const handleEdit = (tenant: Tenant) => {
    setSelectedTenant(tenant);
    setShowModal(true);
  };

  const handleAdd = () => {
    setSelectedTenant(null);
    setShowModal(true);
  };

  const handleSave = (data: Partial<Tenant>) => {
    console.log('Saving tenant:', data);
    setShowModal(false);
    setSelectedTenant(null);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Tenants</h1>
          <p className="text-gray-500">Manage your customers and their subscriptions</p>
        </div>
        <button onClick={handleAdd} className="btn btn-primary">
          <svg
            className="w-5 h-5 mr-2"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 4v16m8-8H4"
            />
          </svg>
          Add Tenant
        </button>
      </div>

      {/* Filters */}
      <div className="card">
        <div className="flex flex-col md:flex-row gap-4">
          <div className="flex-1">
            <div className="relative">
              <svg
                className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                />
              </svg>
              <input
                type="text"
                placeholder="Search tenants..."
                className="input pl-10"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
          </div>
          <div className="flex gap-3">
            <select
              className="input w-auto"
              value={filterPlan}
              onChange={(e) => setFilterPlan(e.target.value)}
            >
              <option value="all">All Plans</option>
              <option value="free">Free</option>
              <option value="starter">Starter</option>
              <option value="professional">Professional</option>
              <option value="enterprise">Enterprise</option>
            </select>
            <select
              className="input w-auto"
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value)}
            >
              <option value="all">All Status</option>
              <option value="active">Active</option>
              <option value="suspended">Suspended</option>
              <option value="deleted">Deleted</option>
            </select>
          </div>
        </div>
      </div>

      {/* Tenants Table */}
      <div className="card overflow-hidden p-0">
        <table className="table">
          <thead>
            <tr>
              <th>Tenant</th>
              <th>Plan</th>
              <th>Status</th>
              <th>Created</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {filteredTenants.map((tenant) => (
              <tr key={tenant.id} className="hover:bg-gray-50">
                <td>
                  <div>
                    <p className="font-medium">{tenant.name}</p>
                    <p className="text-sm text-gray-500">{tenant.email}</p>
                  </div>
                </td>
                <td>
                  <span
                    className={clsx(
                      'badge capitalize',
                      getPlanBadgeClass(tenant.planType)
                    )}
                  >
                    {tenant.planType}
                  </span>
                </td>
                <td>
                  <span
                    className={clsx('badge capitalize', getStatusBadgeClass(tenant.status))}
                  >
                    {tenant.status}
                  </span>
                </td>
                <td className="text-gray-500">
                  {format(new Date(tenant.createdAt), 'MMM d, yyyy')}
                </td>
                <td>
                  <div className="flex space-x-2">
                    <button
                      onClick={() => handleEdit(tenant)}
                      className="p-2 hover:bg-gray-100 rounded-lg"
                      title="Edit"
                    >
                      <svg
                        className="w-4 h-4 text-gray-600"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"
                        />
                      </svg>
                    </button>
                    <button className="p-2 hover:bg-gray-100 rounded-lg" title="View">
                      <svg
                        className="w-4 h-4 text-gray-600"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                        />
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                        />
                      </svg>
                    </button>
                    <button className="p-2 hover:bg-gray-100 rounded-lg" title="Usage">
                      <svg
                        className="w-4 h-4 text-gray-600"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          strokeWidth={2}
                          d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
                        />
                      </svg>
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between">
        <p className="text-sm text-gray-500">
          Showing {filteredTenants.length} of {tenants.length} tenants
        </p>
        <div className="flex space-x-2">
          <button className="btn btn-secondary" disabled>
            Previous
          </button>
          <button className="btn btn-secondary">Next</button>
        </div>
      </div>

      {/* Modal */}
      {showModal && (
        <TenantModal
          tenant={selectedTenant}
          onClose={() => {
            setShowModal(false);
            setSelectedTenant(null);
          }}
          onSave={handleSave}
        />
      )}
    </div>
  );
}

export default Tenants;
