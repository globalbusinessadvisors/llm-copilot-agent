/**
 * Support Page
 *
 * Manage support tickets and customer inquiries.
 */

import React, { useState } from 'react';
import { format } from 'date-fns';
import { clsx } from 'clsx';
import type { Ticket, TicketStatus, TicketPriority } from '../types';

// Mock data
const mockTickets: Ticket[] = [
  {
    id: '1',
    tenantId: '1',
    userId: 'user-1',
    subject: 'API rate limits exceeded unexpectedly',
    description: 'We are hitting rate limits even though our usage is within the plan limits.',
    status: 'open',
    priority: 'high',
    category: 'Technical',
    createdAt: '2024-01-10T08:30:00Z',
    updatedAt: '2024-01-10T08:30:00Z',
  },
  {
    id: '2',
    tenantId: '2',
    userId: 'user-2',
    subject: 'Billing discrepancy on latest invoice',
    description: 'The invoice total does not match our usage records.',
    status: 'in_progress',
    priority: 'medium',
    category: 'Billing',
    assigneeId: 'admin-1',
    createdAt: '2024-01-09T14:20:00Z',
    updatedAt: '2024-01-10T09:15:00Z',
  },
  {
    id: '3',
    tenantId: '3',
    userId: 'user-3',
    subject: 'Cannot access API documentation',
    description: 'The API docs page returns a 404 error.',
    status: 'resolved',
    priority: 'low',
    category: 'Documentation',
    assigneeId: 'admin-2',
    createdAt: '2024-01-08T16:45:00Z',
    updatedAt: '2024-01-09T11:30:00Z',
    resolvedAt: '2024-01-09T11:30:00Z',
  },
  {
    id: '4',
    tenantId: '1',
    userId: 'user-4',
    subject: 'Request for custom model fine-tuning',
    description: 'We need to fine-tune a model for our specific use case.',
    status: 'waiting',
    priority: 'low',
    category: 'Feature Request',
    assigneeId: 'admin-1',
    createdAt: '2024-01-07T10:00:00Z',
    updatedAt: '2024-01-08T14:20:00Z',
  },
  {
    id: '5',
    tenantId: '4',
    userId: 'user-5',
    subject: 'Integration issue with webhook endpoints',
    description: 'Webhooks are not being delivered to our endpoint.',
    status: 'open',
    priority: 'urgent',
    category: 'Technical',
    createdAt: '2024-01-10T09:00:00Z',
    updatedAt: '2024-01-10T09:00:00Z',
  },
];

const tenantNames: Record<string, string> = {
  '1': 'Acme Corporation',
  '2': 'TechStart Inc',
  '3': 'DataFlow Labs',
  '4': 'CloudNative Co',
};

interface TicketDetailProps {
  ticket: Ticket;
  onClose: () => void;
}

function TicketDetail({ ticket, onClose }: TicketDetailProps) {
  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <div className="p-6 border-b border-gray-200">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-bold">Ticket #{ticket.id}</h2>
            <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-lg">
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>
        <div className="p-6 space-y-6">
          <div>
            <h3 className="text-lg font-semibold">{ticket.subject}</h3>
            <div className="flex items-center space-x-4 mt-2 text-sm text-gray-500">
              <span>{tenantNames[ticket.tenantId]}</span>
              <span>{format(new Date(ticket.createdAt), 'MMM d, yyyy h:mm a')}</span>
            </div>
          </div>

          <div className="flex space-x-4">
            <span className={clsx('badge capitalize', {
              'bg-red-100 text-red-800': ticket.priority === 'urgent',
              'bg-orange-100 text-orange-800': ticket.priority === 'high',
              'bg-yellow-100 text-yellow-800': ticket.priority === 'medium',
              'bg-gray-100 text-gray-800': ticket.priority === 'low',
            })}>
              {ticket.priority}
            </span>
            <span className={clsx('badge capitalize', {
              'badge-info': ticket.status === 'open',
              'bg-purple-100 text-purple-800': ticket.status === 'in_progress',
              'bg-yellow-100 text-yellow-800': ticket.status === 'waiting',
              'badge-success': ticket.status === 'resolved',
              'bg-gray-100 text-gray-800': ticket.status === 'closed',
            })}>
              {ticket.status.replace('_', ' ')}
            </span>
            <span className="badge bg-gray-100 text-gray-800">{ticket.category}</span>
          </div>

          <div className="bg-gray-50 p-4 rounded-lg">
            <p className="text-gray-700">{ticket.description}</p>
          </div>

          <div className="border-t pt-6">
            <h4 className="font-semibold mb-4">Update Ticket</h4>
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="label">Status</label>
                  <select className="input" defaultValue={ticket.status}>
                    <option value="open">Open</option>
                    <option value="in_progress">In Progress</option>
                    <option value="waiting">Waiting</option>
                    <option value="resolved">Resolved</option>
                    <option value="closed">Closed</option>
                  </select>
                </div>
                <div>
                  <label className="label">Priority</label>
                  <select className="input" defaultValue={ticket.priority}>
                    <option value="low">Low</option>
                    <option value="medium">Medium</option>
                    <option value="high">High</option>
                    <option value="urgent">Urgent</option>
                  </select>
                </div>
              </div>
              <div>
                <label className="label">Add Response</label>
                <textarea
                  className="input min-h-[100px]"
                  placeholder="Enter your response..."
                />
              </div>
              <div className="flex justify-end space-x-3">
                <button className="btn btn-secondary">Save Draft</button>
                <button className="btn btn-primary">Send Response</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export function Support() {
  const [filterStatus, setFilterStatus] = useState<string>('all');
  const [filterPriority, setFilterPriority] = useState<string>('all');
  const [selectedTicket, setSelectedTicket] = useState<Ticket | null>(null);

  const filteredTickets = mockTickets.filter((ticket) => {
    const matchesStatus = filterStatus === 'all' || ticket.status === filterStatus;
    const matchesPriority = filterPriority === 'all' || ticket.priority === filterPriority;
    return matchesStatus && matchesPriority;
  });

  const openCount = mockTickets.filter((t) => t.status === 'open').length;
  const urgentCount = mockTickets.filter((t) => t.priority === 'urgent' || t.priority === 'high').length;

  const getStatusBadgeClass = (status: TicketStatus) => {
    switch (status) {
      case 'open':
        return 'badge-info';
      case 'in_progress':
        return 'bg-purple-100 text-purple-800';
      case 'waiting':
        return 'bg-yellow-100 text-yellow-800';
      case 'resolved':
        return 'badge-success';
      case 'closed':
        return 'bg-gray-100 text-gray-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const getPriorityBadgeClass = (priority: TicketPriority) => {
    switch (priority) {
      case 'urgent':
        return 'bg-red-100 text-red-800';
      case 'high':
        return 'bg-orange-100 text-orange-800';
      case 'medium':
        return 'bg-yellow-100 text-yellow-800';
      case 'low':
        return 'bg-gray-100 text-gray-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Support Tickets</h1>
          <p className="text-gray-500">Manage customer support requests</p>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="card">
          <p className="text-sm text-gray-500">Open Tickets</p>
          <p className="text-2xl font-bold text-blue-600">{openCount}</p>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">High Priority</p>
          <p className="text-2xl font-bold text-orange-600">{urgentCount}</p>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Avg Response Time</p>
          <p className="text-2xl font-bold">2.4 hrs</p>
        </div>
        <div className="card">
          <p className="text-sm text-gray-500">Resolution Rate</p>
          <p className="text-2xl font-bold text-green-600">94%</p>
        </div>
      </div>

      {/* Filters */}
      <div className="card">
        <div className="flex flex-col md:flex-row gap-4">
          <div className="flex-1">
            <div className="relative">
              <svg className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
              <input
                type="text"
                placeholder="Search tickets..."
                className="input pl-10"
              />
            </div>
          </div>
          <div className="flex gap-3">
            <select
              className="input w-auto"
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value)}
            >
              <option value="all">All Status</option>
              <option value="open">Open</option>
              <option value="in_progress">In Progress</option>
              <option value="waiting">Waiting</option>
              <option value="resolved">Resolved</option>
              <option value="closed">Closed</option>
            </select>
            <select
              className="input w-auto"
              value={filterPriority}
              onChange={(e) => setFilterPriority(e.target.value)}
            >
              <option value="all">All Priority</option>
              <option value="urgent">Urgent</option>
              <option value="high">High</option>
              <option value="medium">Medium</option>
              <option value="low">Low</option>
            </select>
          </div>
        </div>
      </div>

      {/* Tickets List */}
      <div className="card overflow-hidden p-0">
        <table className="table">
          <thead>
            <tr>
              <th>Ticket</th>
              <th>Tenant</th>
              <th>Status</th>
              <th>Priority</th>
              <th>Category</th>
              <th>Created</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {filteredTickets.map((ticket) => (
              <tr key={ticket.id} className="hover:bg-gray-50 cursor-pointer" onClick={() => setSelectedTicket(ticket)}>
                <td>
                  <div>
                    <p className="font-medium">#{ticket.id}</p>
                    <p className="text-sm text-gray-500 truncate max-w-[200px]">{ticket.subject}</p>
                  </div>
                </td>
                <td>{tenantNames[ticket.tenantId]}</td>
                <td>
                  <span className={clsx('badge capitalize', getStatusBadgeClass(ticket.status))}>
                    {ticket.status.replace('_', ' ')}
                  </span>
                </td>
                <td>
                  <span className={clsx('badge capitalize', getPriorityBadgeClass(ticket.priority))}>
                    {ticket.priority}
                  </span>
                </td>
                <td className="text-gray-500">{ticket.category}</td>
                <td className="text-gray-500">
                  {format(new Date(ticket.createdAt), 'MMM d, h:mm a')}
                </td>
                <td>
                  <button
                    className="p-2 hover:bg-gray-100 rounded-lg"
                    onClick={(e) => {
                      e.stopPropagation();
                      setSelectedTicket(ticket);
                    }}
                  >
                    <svg className="w-4 h-4 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                    </svg>
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Ticket Detail Modal */}
      {selectedTicket && (
        <TicketDetail
          ticket={selectedTicket}
          onClose={() => setSelectedTicket(null)}
        />
      )}
    </div>
  );
}

export default Support;
