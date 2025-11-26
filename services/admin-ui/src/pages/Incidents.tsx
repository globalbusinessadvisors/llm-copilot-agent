/**
 * Incidents Page
 *
 * Manage system incidents and status updates.
 */

import React, { useState } from 'react';
import { format } from 'date-fns';
import { clsx } from 'clsx';
import type { Incident, IncidentStatus, IncidentSeverity, IncidentUpdate } from '../types';

// Mock data
const mockIncidents: Incident[] = [
  {
    id: '1',
    title: 'Elevated API Response Times',
    description: 'We are investigating reports of increased latency in API responses across all regions.',
    status: 'monitoring',
    severity: 'minor',
    affectedServices: ['API Gateway', 'AI Service'],
    createdAt: '2024-01-10T08:00:00Z',
    updatedAt: '2024-01-10T10:30:00Z',
    updates: [
      {
        id: 'u1',
        incidentId: '1',
        message: 'We have identified the root cause and implemented a fix. Monitoring for stability.',
        status: 'monitoring',
        createdAt: '2024-01-10T10:30:00Z',
        createdBy: 'Admin',
      },
      {
        id: 'u2',
        incidentId: '1',
        message: 'The issue has been identified as a database connection pool exhaustion.',
        status: 'identified',
        createdAt: '2024-01-10T09:15:00Z',
        createdBy: 'Admin',
      },
      {
        id: 'u3',
        incidentId: '1',
        message: 'We are investigating reports of elevated API response times.',
        status: 'investigating',
        createdAt: '2024-01-10T08:00:00Z',
        createdBy: 'System',
      },
    ],
  },
  {
    id: '2',
    title: 'Webhook Delivery Delays',
    description: 'Some customers may experience delayed webhook deliveries.',
    status: 'resolved',
    severity: 'minor',
    affectedServices: ['Webhook Service'],
    createdAt: '2024-01-08T14:00:00Z',
    updatedAt: '2024-01-08T16:30:00Z',
    resolvedAt: '2024-01-08T16:30:00Z',
    updates: [
      {
        id: 'u4',
        incidentId: '2',
        message: 'The issue has been resolved. All webhooks are now being delivered normally.',
        status: 'resolved',
        createdAt: '2024-01-08T16:30:00Z',
        createdBy: 'Admin',
      },
    ],
  },
  {
    id: '3',
    title: 'Authentication Service Outage',
    description: 'Users were unable to authenticate due to a service failure.',
    status: 'resolved',
    severity: 'critical',
    affectedServices: ['Auth Service', 'API Gateway'],
    createdAt: '2024-01-05T02:00:00Z',
    updatedAt: '2024-01-05T03:45:00Z',
    resolvedAt: '2024-01-05T03:45:00Z',
    updates: [],
  },
];

interface CreateIncidentModalProps {
  onClose: () => void;
  onCreate: (incident: Partial<Incident>) => void;
}

function CreateIncidentModal({ onClose, onCreate }: CreateIncidentModalProps) {
  const [formData, setFormData] = useState({
    title: '',
    description: '',
    severity: 'minor' as IncidentSeverity,
    affectedServices: [] as string[],
  });

  const services = ['API Gateway', 'Auth Service', 'AI Service', 'Webhook Service', 'Database', 'Redis Cache'];

  const handleServiceToggle = (service: string) => {
    setFormData((prev) => ({
      ...prev,
      affectedServices: prev.affectedServices.includes(service)
        ? prev.affectedServices.filter((s) => s !== service)
        : [...prev.affectedServices, service],
    }));
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onCreate({
      ...formData,
      status: 'investigating',
    });
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-xl max-w-lg w-full p-6">
        <h2 className="text-xl font-bold mb-4">Create Incident</h2>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="label">Title</label>
            <input
              type="text"
              className="input"
              value={formData.title}
              onChange={(e) => setFormData({ ...formData, title: e.target.value })}
              required
            />
          </div>
          <div>
            <label className="label">Description</label>
            <textarea
              className="input min-h-[100px]"
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              required
            />
          </div>
          <div>
            <label className="label">Severity</label>
            <select
              className="input"
              value={formData.severity}
              onChange={(e) => setFormData({ ...formData, severity: e.target.value as IncidentSeverity })}
            >
              <option value="minor">Minor</option>
              <option value="major">Major</option>
              <option value="critical">Critical</option>
            </select>
          </div>
          <div>
            <label className="label">Affected Services</label>
            <div className="flex flex-wrap gap-2 mt-2">
              {services.map((service) => (
                <button
                  key={service}
                  type="button"
                  onClick={() => handleServiceToggle(service)}
                  className={clsx(
                    'px-3 py-1 rounded-lg text-sm border transition-colors',
                    formData.affectedServices.includes(service)
                      ? 'bg-primary-100 border-primary-300 text-primary-700'
                      : 'bg-gray-50 border-gray-200 text-gray-600 hover:bg-gray-100'
                  )}
                >
                  {service}
                </button>
              ))}
            </div>
          </div>
          <div className="flex space-x-3 pt-4">
            <button type="button" onClick={onClose} className="btn btn-secondary flex-1">
              Cancel
            </button>
            <button type="submit" className="btn btn-danger flex-1">
              Create Incident
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

interface IncidentDetailProps {
  incident: Incident;
  onClose: () => void;
}

function IncidentDetail({ incident, onClose }: IncidentDetailProps) {
  const [newUpdate, setNewUpdate] = useState('');
  const [newStatus, setNewStatus] = useState<IncidentStatus>(incident.status);

  const getStatusBadgeClass = (status: IncidentStatus) => {
    switch (status) {
      case 'investigating':
        return 'bg-red-100 text-red-800';
      case 'identified':
        return 'bg-orange-100 text-orange-800';
      case 'monitoring':
        return 'bg-yellow-100 text-yellow-800';
      case 'resolved':
        return 'badge-success';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <div className="p-6 border-b border-gray-200">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <span className={clsx('badge capitalize', getStatusBadgeClass(incident.status))}>
                {incident.status}
              </span>
              <span className={clsx('badge capitalize', {
                'bg-gray-100 text-gray-800': incident.severity === 'minor',
                'bg-orange-100 text-orange-800': incident.severity === 'major',
                'bg-red-100 text-red-800': incident.severity === 'critical',
              })}>
                {incident.severity}
              </span>
            </div>
            <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-lg">
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
          <h2 className="text-xl font-bold mt-4">{incident.title}</h2>
          <p className="text-gray-500 mt-2">{incident.description}</p>
          <div className="flex flex-wrap gap-2 mt-4">
            {incident.affectedServices.map((service) => (
              <span key={service} className="badge bg-gray-100 text-gray-700">{service}</span>
            ))}
          </div>
        </div>

        <div className="p-6 border-b border-gray-200">
          <h3 className="font-semibold mb-4">Post Update</h3>
          <div className="space-y-4">
            <div>
              <label className="label">Status</label>
              <select
                className="input"
                value={newStatus}
                onChange={(e) => setNewStatus(e.target.value as IncidentStatus)}
              >
                <option value="investigating">Investigating</option>
                <option value="identified">Identified</option>
                <option value="monitoring">Monitoring</option>
                <option value="resolved">Resolved</option>
              </select>
            </div>
            <div>
              <label className="label">Update Message</label>
              <textarea
                className="input min-h-[80px]"
                value={newUpdate}
                onChange={(e) => setNewUpdate(e.target.value)}
                placeholder="Provide an update on the incident..."
              />
            </div>
            <button className="btn btn-primary">Post Update</button>
          </div>
        </div>

        <div className="p-6">
          <h3 className="font-semibold mb-4">Timeline</h3>
          <div className="space-y-4">
            {incident.updates.map((update, index) => (
              <div key={update.id} className="flex space-x-4">
                <div className="flex flex-col items-center">
                  <div className={clsx('w-3 h-3 rounded-full', {
                    'bg-green-500': update.status === 'resolved',
                    'bg-yellow-500': update.status === 'monitoring',
                    'bg-orange-500': update.status === 'identified',
                    'bg-red-500': update.status === 'investigating',
                  })} />
                  {index < incident.updates.length - 1 && (
                    <div className="w-0.5 h-full bg-gray-200 mt-1" />
                  )}
                </div>
                <div className="flex-1 pb-4">
                  <div className="flex items-center justify-between">
                    <span className={clsx('badge capitalize text-xs', getStatusBadgeClass(update.status))}>
                      {update.status}
                    </span>
                    <span className="text-xs text-gray-400">
                      {format(new Date(update.createdAt), 'MMM d, h:mm a')}
                    </span>
                  </div>
                  <p className="text-sm text-gray-700 mt-1">{update.message}</p>
                  <p className="text-xs text-gray-400 mt-1">by {update.createdBy}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

export function Incidents() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [selectedIncident, setSelectedIncident] = useState<Incident | null>(null);
  const [filterStatus, setFilterStatus] = useState<string>('all');

  const filteredIncidents = mockIncidents.filter((incident) =>
    filterStatus === 'all' ? true : incident.status === filterStatus
  );

  const activeIncidents = mockIncidents.filter((i) => i.status !== 'resolved').length;

  const getStatusBadgeClass = (status: IncidentStatus) => {
    switch (status) {
      case 'investigating':
        return 'bg-red-100 text-red-800';
      case 'identified':
        return 'bg-orange-100 text-orange-800';
      case 'monitoring':
        return 'bg-yellow-100 text-yellow-800';
      case 'resolved':
        return 'badge-success';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const getSeverityBadgeClass = (severity: IncidentSeverity) => {
    switch (severity) {
      case 'critical':
        return 'bg-red-100 text-red-800';
      case 'major':
        return 'bg-orange-100 text-orange-800';
      case 'minor':
        return 'bg-gray-100 text-gray-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Incidents</h1>
          <p className="text-gray-500">Manage system incidents and status updates</p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn btn-danger">
          <svg className="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          Create Incident
        </button>
      </div>

      {/* Current Status */}
      <div className={clsx('card border-l-4', activeIncidents > 0 ? 'border-l-yellow-500' : 'border-l-green-500')}>
        <div className="flex items-center space-x-4">
          <div className={clsx('w-12 h-12 rounded-full flex items-center justify-center', activeIncidents > 0 ? 'bg-yellow-100' : 'bg-green-100')}>
            {activeIncidents > 0 ? (
              <svg className="w-6 h-6 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            ) : (
              <svg className="w-6 h-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
            )}
          </div>
          <div>
            <h3 className="font-semibold">
              {activeIncidents > 0 ? `${activeIncidents} Active Incident${activeIncidents > 1 ? 's' : ''}` : 'All Systems Operational'}
            </h3>
            <p className="text-sm text-gray-500">
              {activeIncidents > 0 ? 'Some services may be affected' : 'No ongoing incidents'}
            </p>
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="flex space-x-3">
        <select
          className="input w-auto"
          value={filterStatus}
          onChange={(e) => setFilterStatus(e.target.value)}
        >
          <option value="all">All Status</option>
          <option value="investigating">Investigating</option>
          <option value="identified">Identified</option>
          <option value="monitoring">Monitoring</option>
          <option value="resolved">Resolved</option>
        </select>
      </div>

      {/* Incidents List */}
      <div className="space-y-4">
        {filteredIncidents.map((incident) => (
          <div
            key={incident.id}
            className="card cursor-pointer hover:shadow-md transition-shadow"
            onClick={() => setSelectedIncident(incident)}
          >
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <div className="flex items-center space-x-3 mb-2">
                  <span className={clsx('badge capitalize', getStatusBadgeClass(incident.status))}>
                    {incident.status}
                  </span>
                  <span className={clsx('badge capitalize', getSeverityBadgeClass(incident.severity))}>
                    {incident.severity}
                  </span>
                </div>
                <h3 className="font-semibold">{incident.title}</h3>
                <p className="text-sm text-gray-500 mt-1">{incident.description}</p>
                <div className="flex flex-wrap gap-2 mt-3">
                  {incident.affectedServices.map((service) => (
                    <span key={service} className="text-xs bg-gray-100 text-gray-600 px-2 py-1 rounded">
                      {service}
                    </span>
                  ))}
                </div>
              </div>
              <div className="text-right text-sm text-gray-500">
                <p>{format(new Date(incident.createdAt), 'MMM d, h:mm a')}</p>
                {incident.resolvedAt && (
                  <p className="text-green-600">Resolved {format(new Date(incident.resolvedAt), 'MMM d')}</p>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Modals */}
      {showCreateModal && (
        <CreateIncidentModal
          onClose={() => setShowCreateModal(false)}
          onCreate={(data) => {
            console.log('Create incident:', data);
            setShowCreateModal(false);
          }}
        />
      )}

      {selectedIncident && (
        <IncidentDetail
          incident={selectedIncident}
          onClose={() => setSelectedIncident(null)}
        />
      )}
    </div>
  );
}

export default Incidents;
