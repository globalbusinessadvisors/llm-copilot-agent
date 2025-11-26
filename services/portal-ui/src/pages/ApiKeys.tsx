/**
 * API Keys Page
 *
 * Manage API keys for accessing the platform.
 */

import React, { useState } from 'react';
import { format } from 'date-fns';
import { clsx } from 'clsx';

interface ApiKey {
  id: string;
  name: string;
  prefix: string;
  status: 'active' | 'revoked';
  createdAt: string;
  lastUsedAt?: string;
  expiresAt?: string;
}

const mockApiKeys: ApiKey[] = [
  {
    id: '1',
    name: 'Production',
    prefix: 'sk-prod-abc12',
    status: 'active',
    createdAt: '2023-11-15T10:00:00Z',
    lastUsedAt: '2024-01-10T08:30:00Z',
  },
  {
    id: '2',
    name: 'Development',
    prefix: 'sk-dev-xyz89',
    status: 'active',
    createdAt: '2023-12-01T14:00:00Z',
    lastUsedAt: '2024-01-09T16:45:00Z',
  },
  {
    id: '3',
    name: 'Testing',
    prefix: 'sk-test-def45',
    status: 'revoked',
    createdAt: '2023-10-20T09:00:00Z',
  },
];

interface CreateKeyModalProps {
  onClose: () => void;
  onCreate: (name: string) => void;
}

function CreateKeyModal({ onClose, onCreate }: CreateKeyModalProps) {
  const [name, setName] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onCreate(name);
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-xl max-w-md w-full p-6">
        <h2 className="text-xl font-bold mb-4">Create API Key</h2>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="label">Key Name</label>
            <input
              type="text"
              className="input"
              placeholder="e.g., Production, Development"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
            <p className="text-sm text-gray-500 mt-1">
              A descriptive name to identify this API key
            </p>
          </div>
          <div className="flex space-x-3 pt-4">
            <button type="button" onClick={onClose} className="btn btn-secondary flex-1">
              Cancel
            </button>
            <button type="submit" className="btn btn-primary flex-1">
              Create Key
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

interface ShowKeyModalProps {
  apiKey: string;
  onClose: () => void;
}

function ShowKeyModal({ apiKey, onClose }: ShowKeyModalProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(apiKey);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
      <div className="bg-white rounded-xl max-w-lg w-full p-6">
        <div className="flex items-center space-x-3 mb-4">
          <div className="w-10 h-10 bg-green-100 rounded-full flex items-center justify-center">
            <svg className="w-5 h-5 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
          </div>
          <h2 className="text-xl font-bold">API Key Created</h2>
        </div>

        <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4 mb-4">
          <p className="text-sm text-yellow-800">
            <strong>Important:</strong> This is the only time you will see this API key.
            Please copy it and store it securely.
          </p>
        </div>

        <div className="relative">
          <input
            type="text"
            className="input font-mono text-sm pr-24"
            value={apiKey}
            readOnly
          />
          <button
            onClick={handleCopy}
            className="absolute right-2 top-1/2 transform -translate-y-1/2 px-3 py-1 bg-gray-100 hover:bg-gray-200 rounded text-sm font-medium"
          >
            {copied ? 'Copied!' : 'Copy'}
          </button>
        </div>

        <button onClick={onClose} className="btn btn-primary w-full mt-6">
          Done
        </button>
      </div>
    </div>
  );
}

export function ApiKeys() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newApiKey, setNewApiKey] = useState<string | null>(null);

  const handleCreate = (name: string) => {
    // In production, this would call the API
    const generatedKey = `sk-${name.toLowerCase().substring(0, 4)}-${Math.random().toString(36).substring(2, 15)}${Math.random().toString(36).substring(2, 15)}`;
    setNewApiKey(generatedKey);
    setShowCreateModal(false);
  };

  const handleRevoke = (keyId: string) => {
    if (confirm('Are you sure you want to revoke this API key? This action cannot be undone.')) {
      console.log('Revoking key:', keyId);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">API Keys</h1>
          <p className="text-gray-500">Manage your API keys for accessing the platform</p>
        </div>
        <button onClick={() => setShowCreateModal(true)} className="btn btn-primary">
          <svg className="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          Create Key
        </button>
      </div>

      {/* Security Notice */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
        <div className="flex items-start space-x-3">
          <svg className="w-5 h-5 text-blue-600 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <div>
            <p className="font-medium text-blue-900">Keep your API keys secure</p>
            <p className="text-sm text-blue-700 mt-1">
              Never share your API keys or commit them to version control. Use environment
              variables to store them securely.
            </p>
          </div>
        </div>
      </div>

      {/* API Keys Table */}
      <div className="card overflow-hidden p-0">
        <table className="table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Key</th>
              <th>Status</th>
              <th>Last Used</th>
              <th>Created</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {mockApiKeys.map((key) => (
              <tr key={key.id}>
                <td className="font-medium">{key.name}</td>
                <td>
                  <code className="text-sm bg-gray-100 px-2 py-1 rounded">
                    {key.prefix}...
                  </code>
                </td>
                <td>
                  <span
                    className={clsx(
                      'badge capitalize',
                      key.status === 'active' ? 'badge-success' : 'bg-gray-100 text-gray-600'
                    )}
                  >
                    {key.status}
                  </span>
                </td>
                <td className="text-gray-500">
                  {key.lastUsedAt
                    ? format(new Date(key.lastUsedAt), 'MMM d, yyyy')
                    : 'Never'}
                </td>
                <td className="text-gray-500">
                  {format(new Date(key.createdAt), 'MMM d, yyyy')}
                </td>
                <td>
                  {key.status === 'active' && (
                    <button
                      onClick={() => handleRevoke(key.id)}
                      className="text-red-600 hover:text-red-700 text-sm font-medium"
                    >
                      Revoke
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Usage Instructions */}
      <div className="card">
        <h3 className="text-lg font-semibold mb-4">Quick Start</h3>
        <p className="text-gray-600 mb-4">
          Use your API key to authenticate requests to our API:
        </p>
        <div className="bg-gray-900 rounded-lg p-4 overflow-x-auto">
          <pre className="text-sm text-gray-100">
            <code>{`curl -X POST https://api.llmcopilot.com/v1/chat/completions \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "model": "claude-3-sonnet",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'`}</code>
          </pre>
        </div>
        <div className="mt-4">
          <a href="/docs" className="text-primary-600 hover:underline text-sm">
            View full API documentation →
          </a>
        </div>
      </div>

      {/* Modals */}
      {showCreateModal && (
        <CreateKeyModal
          onClose={() => setShowCreateModal(false)}
          onCreate={handleCreate}
        />
      )}

      {newApiKey && (
        <ShowKeyModal
          apiKey={newApiKey}
          onClose={() => setNewApiKey(null)}
        />
      )}
    </div>
  );
}

export default ApiKeys;
